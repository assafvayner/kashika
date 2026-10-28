//! A self-contained approximation of the `through`, `cross` and `near` checks, used until
//! `kashika-check` exists (M3). Task 15 replaces it.

use kashika_geom::{flatten, length, Pt, Rect, TOLERANCE};
use kashika_model::{Line, Scene, ShapeKind};

use crate::{Corridor, Crossing};

pub struct Ctx<'a> {
    pub start_owner: Option<&'a str>,
    pub end_owner: Option<&'a str>,
    /// A line to ignore as an obstacle: the one being rerouted.
    pub ignore: Option<&'a str>,
}

pub struct Score {
    pub crossed: Vec<Crossing>,
    pub near: usize,
    pub length: f64,
}

/// Scores `probe` against the rest of `scene`. `None` means it enters a box.
pub fn score(scene: &Scene, probe: &Line, ctx: &Ctx) -> Option<Score> {
    let last = probe.path.len().checked_sub(1)?;
    let pieces: Vec<(usize, Pt, Pt)> = probe
        .path
        .iter()
        .enumerate()
        .flat_map(|(i, seg)| {
            flatten(std::slice::from_ref(seg))
                .windows(2)
                .map(|w| (i, w[0], w[1]))
                .collect::<Vec<_>>()
        })
        .collect();
    let exempt = |i: usize, id: &str| {
        (i == 0 && ctx.start_owner == Some(id)) || (i == last && ctx.end_owner == Some(id))
    };
    let mut near = 0;
    for s in scene.shapes.iter().filter(|s| s.kind == ShapeKind::Box) {
        let inner = inset(s.rect, TOLERANCE);
        let outer = inset(s.rect, -(scene.clearance - TOLERANCE));
        let mut close = false;
        for &(i, a, b) in &pieces {
            if exempt(i, &s.id) {
                continue;
            }
            if clips(inner, a, b) {
                return None;
            }
            close |= clips(outer, a, b);
        }
        near += usize::from(close);
    }
    let ends = [probe.path[0].from(), probe.path[last].to()];
    let mut crossed: Vec<Crossing> = Vec::new();
    for l in scene
        .lines
        .iter()
        .filter(|l| l.id != probe.id && Some(l.id.as_str()) != ctx.ignore)
    {
        let other = flatten(&l.path);
        let mut alongside = false;
        for &(_, a, b) in &pieces {
            for w in other.windows(2) {
                if let Some(p) = intersect(a, b, w[0], w[1]) {
                    let shared = ends.iter().any(|e| e.dist(p) <= TOLERANCE)
                        || scene.junctions.iter().any(|j| j.at.dist(p) <= TOLERANCE);
                    let seen = crossed
                        .iter()
                        .any(|c| c.line == l.id && c.at.dist(p) <= TOLERANCE);
                    if !shared && !seen {
                        crossed.push(Crossing {
                            line: l.id.clone(),
                            at: p,
                        });
                    }
                }
                alongside |= runs_alongside(a, b, w[0], w[1], scene.clearance);
            }
        }
        near += usize::from(alongside);
    }
    Some(Score {
        crossed,
        near,
        length: length(&probe.path),
    })
}

/// The free band around a lane: between the nearest box edges on either side, among the
/// boxes that overlap the lane's span.
pub fn corridor(scene: &Scene, axis: &'static str, value: f64, span: (f64, f64)) -> Corridor {
    let (s0, s1) = (span.0.min(span.1), span.0.max(span.1));
    let mut lo = 0.0_f64;
    let mut hi = if axis == "x" {
        scene.size.0
    } else {
        scene.size.1
    };
    for s in scene.shapes.iter().filter(|s| s.kind == ShapeKind::Box) {
        let r = s.rect;
        let (a0, a1, b0, b1) = if axis == "y" {
            (r.y, r.bottom(), r.x, r.right())
        } else {
            (r.x, r.right(), r.y, r.bottom())
        };
        if b1 <= s0 || b0 >= s1 {
            continue;
        }
        if a1 <= value {
            lo = lo.max(a1);
        }
        if a0 >= value {
            hi = hi.min(a0);
        }
    }
    Corridor {
        axis,
        band: [lo, hi],
    }
}

fn inset(r: Rect, d: f64) -> Rect {
    Rect::new(r.x + d, r.y + d, r.w - 2.0 * d, r.h - 2.0 * d)
}

/// Liang–Barsky: does the segment a–b pass through the open rectangle `r`?
fn clips(r: Rect, a: Pt, b: Pt) -> bool {
    if r.w <= 0.0 || r.h <= 0.0 {
        return false;
    }
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-d.x, a.x - r.x),
        (d.x, r.right() - a.x),
        (-d.y, a.y - r.y),
        (d.y, r.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q <= 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 >= t1 {
            return false;
        }
    }
    true
}

/// Intersection point of segments a–b and c–d, rounded to 0.01px; `None` when parallel.
fn intersect(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let (r, s) = (b - a, d - c);
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-9 {
        return None;
    }
    let w = c - a;
    let t = (w.x * s.y - w.y * s.x) / den;
    let u = (w.x * r.y - w.y * r.x) / den;
    let inside = |v: f64| (-1e-9..=1.0 + 1e-9).contains(&v);
    (inside(t) && inside(u)).then(|| {
        let p = a + r * t;
        Pt::new((p.x * 100.0).round() / 100.0, (p.y * 100.0).round() / 100.0)
    })
}

/// Two parallel axis-aligned pieces closer than `clearance` that overlap by more than 20px.
fn runs_alongside(a: Pt, b: Pt, c: Pt, d: Pt, clearance: f64) -> bool {
    let horizontal = |p: Pt, q: Pt| (p.y - q.y).abs() <= TOLERANCE && (p.x - q.x).abs() > TOLERANCE;
    let vertical = |p: Pt, q: Pt| (p.x - q.x).abs() <= TOLERANCE && (p.y - q.y).abs() > TOLERANCE;
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))
    };
    if horizontal(a, b) && horizontal(c, d) {
        (a.y - c.y).abs() < clearance && overlap(a.x, b.x, c.x, d.x) > 20.0
    } else if vertical(a, b) && vertical(c, d) {
        (a.x - c.x).abs() < clearance && overlap(a.y, b.y, c.y, d.y) > 20.0
    } else {
        false
    }
}
