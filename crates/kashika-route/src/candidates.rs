use kashika_check::ctx::Ctx;
use kashika_check::free::{Axis, free_intervals};
use kashika_geom::{Hit, Pt, Rect, TOLERANCE, dot, inside_runs, seg_hit};
use kashika_model::{Scene, ShapeKind};

use crate::text::num;
use crate::{Endpoint, SQUARE, Style};

/// Lanes per axis that detours pick from: those nearest either endpoint.
pub const DETOUR_LANES: usize = 16;
/// Detours scored with `kashika check`: the best of a geometric pre-pass.
pub const MAX_DETOURS: usize = 32;

/// An `x` or `y` target: relative to a box corner where possible, otherwise a literal.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    pub value: f64,
    pub expr: Option<String>,
}

/// A `curve via` control point.
#[derive(Debug, Clone, PartialEq)]
pub struct Ctrl {
    pub at: Pt,
    pub expr: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Hop {
    /// Straight to the end point.
    To,
    /// `hv <end>`
    Hv,
    /// `vh <end>`
    Vh,
    X(Lane),
    Y(Lane),
    Curve {
        left: bool,
        amount: f64,
    },
    Via(Ctrl, Ctrl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub hops: Vec<Hop>,
    /// The lane the path is routed along: `("x", v)` when a vertical segment runs at x=v. For
    /// a detour, its second lane.
    pub lane: Option<(&'static str, f64)>,
}

pub fn generate(scene: &Scene, from: &Endpoint, to: &Endpoint, style: Style) -> Vec<Plan> {
    generate_skipping(scene, from, to, style, &[])
}

/// Like [`generate`]; lines in `skip` aren't obstacles when picking detours.
pub fn generate_skipping(
    scene: &Scene,
    from: &Endpoint,
    to: &Endpoint,
    style: Style,
    skip: &[&str],
) -> Vec<Plan> {
    match style {
        Style::Bezier => curves(from, to),
        Style::Corner | Style::Arc => orthogonal(scene, from, to, skip),
    }
}

/// The corners of an orthogonal plan from `a` to `b`; `None` for curves.
pub fn corners(a: Pt, b: Pt, plan: &Plan) -> Option<Vec<Pt>> {
    let mut pts = vec![a];
    for hop in &plan.hops {
        let at = pts[pts.len() - 1];
        match hop {
            Hop::To => pts.push(b),
            Hop::Hv => pts.extend([Pt::new(b.x, at.y), b]),
            Hop::Vh => pts.extend([Pt::new(at.x, b.y), b]),
            Hop::X(l) => pts.push(Pt::new(l.value, at.y)),
            Hop::Y(l) => pts.push(Pt::new(at.x, l.value)),
            Hop::Curve { .. } | Hop::Via(..) => return None,
        }
    }
    Some(pts)
}

fn orthogonal(scene: &Scene, a: &Endpoint, b: &Endpoint, skip: &[&str]) -> Vec<Plan> {
    let simple = |hops: Vec<Hop>| Plan { hops, lane: None };
    let mut out = Vec::new();
    if (a.at.x - b.at.x).abs() <= 0.5 || (a.at.y - b.at.y).abs() <= 0.5 {
        out.push(simple(vec![Hop::To]));
    }
    out.push(simple(vec![Hop::Hv]));
    out.push(simple(vec![Hop::Vh]));
    for lane in lanes(scene, a.at.x, b.at.x, "x") {
        let v = lane.value;
        out.push(Plan {
            hops: vec![Hop::X(lane), Hop::Vh],
            lane: Some(("x", v)),
        });
    }
    for lane in lanes(scene, a.at.y, b.at.y, "y") {
        let v = lane.value;
        out.push(Plan {
            hops: vec![Hop::Y(lane), Hop::Hv],
            lane: Some(("y", v)),
        });
    }
    out.extend(detours(scene, a, b, skip));
    out
}

/// Three- and four-elbow plans through two or three lanes: `x -> y -> hv` and `y -> x -> vh`
/// end on the other axis than they start, `x -> y -> x -> vh` and `y -> x -> y -> hv` on the
/// same one. A geometric pre-pass drops paths that enter a box or leave or enter a side
/// anchor the wrong way, and keeps the [`MAX_DETOURS`] best by estimated crossings,
/// near-misses, segments and length.
fn detours(scene: &Scene, a: &Endpoint, b: &Endpoint, skip: &[&str]) -> Vec<Plan> {
    let ctx = Ctx::new(scene);
    let xs = detour_lanes(&ctx, a, b, Axis::X, skip);
    let ys = detour_lanes(&ctx, a, b, Axis::Y, skip);
    let field = Field::new(&ctx, a, b, skip);
    let mut found: Vec<(Estimate, Plan)> = Vec::new();
    let mut consider = |hops: Vec<Hop>, lane: (&'static str, f64)| {
        let plan = Plan {
            hops,
            lane: Some(lane),
        };
        let pts = corners(a.at, b.at, &plan).expect("orthogonal hops");
        if let Some(e) = field.estimate(&pts) {
            found.push((e, plan));
        }
    };
    for (lead, back) in [("x", "y"), ("y", "x")] {
        let (outer, inner) = if lead == "x" { (&xs, &ys) } else { (&ys, &xs) };
        let (cross, along) = if lead == "x" {
            (Hop::Hv, Hop::Vh)
        } else {
            (Hop::Vh, Hop::Hv)
        };
        for l1 in outer {
            for l2 in inner {
                let lane = (back, l2.value);
                consider(vec![hop(lead, l1), hop(back, l2), cross.clone()], lane);
                for l3 in outer {
                    consider(
                        vec![hop(lead, l1), hop(back, l2), hop(lead, l3), along.clone()],
                        lane,
                    );
                }
            }
        }
    }
    found.sort_by_key(|f| f.0);
    found
        .into_iter()
        .take(MAX_DETOURS)
        .map(|(_, plan)| plan)
        .collect()
}

fn hop(axis: &str, lane: &Lane) -> Hop {
    if axis == "x" {
        Hop::X(lane.clone())
    } else {
        Hop::Y(lane.clone())
    }
}

/// Lanes for detours on one axis: the box lanes of [`lanes`], plus the middle of each free
/// interval (as `inspect --corridors` reports between the endpoints) that holds none of them,
/// at least `clearance` inside the canvas. The [`DETOUR_LANES`] nearest either endpoint are kept.
fn detour_lanes(ctx: &Ctx, a: &Endpoint, b: &Endpoint, axis: Axis, skip: &[&str]) -> Vec<Lane> {
    let scene = ctx.scene;
    let (pa, pb, band, limit) = match axis {
        Axis::X => (a.at.x, b.at.x, (a.at.y, b.at.y), scene.size.0),
        Axis::Y => (a.at.y, b.at.y, (a.at.x, b.at.x), scene.size.1),
    };
    let mut pool = lanes(scene, pa, pb, axis.name());
    for (lo, hi) in free_intervals(ctx, axis, band, (0.0, limit), skip) {
        if pool.iter().any(|l| lo <= l.value && l.value <= hi) {
            continue;
        }
        let v = snap(scene, (lo + hi) / 2.0);
        let v = if lo <= v && v <= hi {
            v
        } else {
            (lo + hi) / 2.0
        };
        pool.push(Lane {
            value: v,
            expr: None,
        });
    }
    let margin = scene.clearance.max(TOLERANCE);
    pool.retain(|l| l.value >= margin && l.value <= limit - margin);
    pool.sort_by_key(|l| l.expr.is_none());
    let mut out: Vec<Lane> = Vec::new();
    for l in pool {
        if !out.iter().any(|o| (o.value - l.value).abs() <= 0.5) {
            out.push(l);
        }
    }
    let away = |v: f64| (v - pa).abs().min((v - pb).abs());
    out.sort_by(|p, q| away(p.value).total_cmp(&away(q.value)));
    out.truncate(DETOUR_LANES);
    out
}

fn snap(scene: &Scene, v: f64) -> f64 {
    match scene.grid {
        Some(grid) if grid > 0.0 => (v / grid).round() * grid,
        _ => v.round(),
    }
}

/// Estimated crossings, near-misses and segments, then length: the order `route` ranks by.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Estimate(usize, usize, usize, f64);

impl Eq for Estimate {}

impl PartialOrd for Estimate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Estimate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0, self.1, self.2)
            .cmp(&(other.0, other.1, other.2))
            .then(self.3.total_cmp(&other.3))
    }
}

/// The obstacles a detour's corners are measured against.
struct Field<'a> {
    boxes: Vec<(&'a str, Rect, &'a [Pt])>,
    pieces: Vec<(Pt, Pt)>,
    starts: Vec<&'a str>,
    ends: Vec<&'a str>,
    from: &'a Endpoint,
    to: &'a Endpoint,
    clearance: f64,
}

impl<'a> Field<'a> {
    fn new(ctx: &'a Ctx, from: &'a Endpoint, to: &'a Endpoint, skip: &[&str]) -> Self {
        Field {
            boxes: ctx
                .boxes()
                .map(|s| (s.shape.id.as_str(), s.shape.rect, s.poly.as_slice()))
                .collect(),
            pieces: ctx
                .lines
                .iter()
                .filter(|l| !skip.contains(&l.line.id.as_str()))
                .flat_map(|l| l.pieces.iter().map(|&(p, q, _)| (p, q)))
                .collect(),
            starts: ctx.attached(from.at),
            ends: ctx.attached(to.at),
            from,
            to,
            clearance: ctx.scene.clearance,
        }
    }

    /// `None` when the path doubles back on a corner, leaves or enters a side anchor the wrong
    /// way, or enters a box. The first and last segments may touch the boxes they attach to.
    fn estimate(&self, pts: &[Pt]) -> Option<Estimate> {
        let n = pts.len() - 1;
        if pts.windows(2).any(|w| w[0].dist(w[1]) <= TOLERANCE) {
            return None;
        }
        let dir = |i: usize| (pts[i + 1] - pts[i]).norm();
        if self
            .from
            .side
            .is_some_and(|s| dot(dir(0), s.outward()) <= SQUARE)
            || self
                .to
                .side
                .is_some_and(|s| dot(dir(n - 1), s.outward()) >= -SQUARE)
        {
            return None;
        }
        let c = self.clearance;
        let (mut crossings, mut near, mut length) = (0, 0, 0.0);
        for (i, w) in pts.windows(2).enumerate() {
            let (p, q) = (w[0], w[1]);
            length += p.dist(q);
            let (lo, hi) = (
                Pt::new(p.x.min(q.x), p.y.min(q.y)),
                Pt::new(p.x.max(q.x), p.y.max(q.y)),
            );
            for &(id, r, poly) in &self.boxes {
                if (i == 0 && self.starts.contains(&id)) || (i == n - 1 && self.ends.contains(&id))
                {
                    continue;
                }
                if hi.x <= r.x - c
                    || lo.x >= r.right() + c
                    || hi.y <= r.y - c
                    || lo.y >= r.bottom() + c
                {
                    continue;
                }
                if !inside_runs(p, q, poly).is_empty() {
                    return None;
                }
                near += 1;
            }
            for &(u, v) in &self.pieces {
                match seg_hit(p, q, u, v, 0.0) {
                    Some(Hit::Point(x))
                        if x.dist(self.from.at) > TOLERANCE && x.dist(self.to.at) > TOLERANCE =>
                    {
                        crossings += 1
                    }
                    Some(Hit::Overlap(..)) => near += 1,
                    _ => {}
                }
            }
        }
        Some(Estimate(crossings, near, n, length))
    }
}

/// Lane positions on one axis: the midpoint between the endpoints, and `2 × clearance`
/// outside each box edge, written relative to the box's corner so the lane follows the box.
fn lanes(scene: &Scene, a: f64, b: f64, axis: &str) -> Vec<Lane> {
    let g = 2.0 * scene.clearance;
    let gap = num(g);
    let limit = if axis == "x" {
        scene.size.0
    } else {
        scene.size.1
    };
    let mid = match scene.grid {
        Some(grid) if grid > 0.0 => ((a + b) / 2.0 / grid).round() * grid,
        _ => ((a + b) / 2.0).round(),
    };
    let mut out = vec![Lane {
        value: mid,
        expr: None,
    }];
    for s in scene
        .shapes
        .iter()
        .filter(|s| s.kind != ShapeKind::Group && s.layer.is_normal())
    {
        let (r, id) = (s.rect, &s.id);
        if axis == "x" {
            out.push(Lane {
                value: r.x - g,
                expr: Some(format!("{id}.tl+(-{gap},0)")),
            });
            out.push(Lane {
                value: r.right() + g,
                expr: Some(format!("{id}.tr+({gap},0)")),
            });
        } else {
            out.push(Lane {
                value: r.y - g,
                expr: Some(format!("{id}.tl+(0,-{gap})")),
            });
            out.push(Lane {
                value: r.bottom() + g,
                expr: Some(format!("{id}.bl+(0,{gap})")),
            });
        }
    }
    out.retain(|l| l.value > 0.0 && l.value < limit);
    out
}

fn curves(a: &Endpoint, b: &Endpoint) -> Vec<Plan> {
    let d = a.at.dist(b.at);
    let mut out = Vec::new();
    for amount in [(d * 0.15).round(), (d * 0.3).round()] {
        if amount >= 1.0 {
            for left in [true, false] {
                out.push(Plan {
                    hops: vec![Hop::Curve { left, amount }],
                    lane: None,
                });
            }
        }
    }
    if a.side.is_some() || b.side.is_some() {
        let k = (d / 3.0).max(20.0).round();
        out.push(Plan {
            hops: vec![Hop::Via(ctrl(a, k), ctrl(b, k))],
            lane: None,
        });
    }
    out
}

/// A control point `k` px out from an endpoint along its side's normal, written as an offset
/// from the endpoint's reference when that reference has no offset of its own.
fn ctrl(e: &Endpoint, k: f64) -> Ctrl {
    let Some(side) = e.side else {
        return Ctrl {
            at: e.at,
            expr: e.expr.clone(),
        };
    };
    let off = side.outward() * k;
    let expr = e
        .expr
        .as_ref()
        .filter(|x| !x.contains('+') && !x.contains("-("))
        .map(|x| format!("{x}+({},{})", num(off.x), num(off.y)));
    Ctrl {
        at: e.at + off,
        expr,
    }
}
