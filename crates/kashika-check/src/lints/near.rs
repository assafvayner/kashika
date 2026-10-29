use kashika_diag::{Diagnostic, code};
use kashika_geom::{Pt, TOLERANCE, cross, dot, edges, fmt_num, inside_runs, round2, seg_seg_dist};

use crate::ctx::{Ctx, LineGeom, ShapeGeom, kind_name, line_el, shape_el};
use crate::free::with_free;
use crate::sink::Sink;

/// Pieces whose directions differ by less than ~3° count as parallel.
const PARALLEL: f64 = 0.05;
/// Parallel lines must run side by side for longer than this to be `near`.
const MIN_RUN: f64 = 20.0;

/// `near`: a line runs within `clearance` of a box edge, along a group border, or along a
/// parallel line for more than 20px.
pub fn near(ctx: &Ctx, out: &mut Sink) {
    near_boxes(ctx, out);
    near_groups(ctx, out);
    near_lines(ctx, out);
}

/// Perpendicular gap and overlap length of two nearly parallel segments that overlap along their direction.
fn parallel(p: Pt, q: Pt, e0: Pt, e1: Pt) -> Option<(f64, f64)> {
    let le = e0.dist(e1);
    if le <= 1e-9 || p.dist(q) <= 1e-9 {
        return None;
    }
    let (u, v) = ((q - p).norm(), (e1 - e0).norm());
    if cross(u, v).abs() > PARALLEL {
        return None;
    }
    let (s0, s1) = (dot(p - e0, v), dot(q - e0, v));
    let (lo, hi) = (s0.min(s1).max(0.0), s0.max(s1).min(le));
    if hi - lo <= TOLERANCE {
        return None;
    }
    let gap = cross(u, e0 + v * ((lo + hi) / 2.0) - p).abs();
    Some((gap, hi - lo))
}

/// How far apart two bounding boxes can be while pieces in them are within `c`: a parallel
/// gap is measured square to one piece, so allow some slack.
fn reach(c: f64) -> f64 {
    2.0 * c + 1.0
}

fn near_boxes(ctx: &Ctx, out: &mut Sink) {
    let c = ctx.scene.clearance;
    for (i, l) in ctx.lines.iter().enumerate() {
        let mut own = ctx.attached(l.start);
        own.extend(ctx.attached(l.end));
        for s in ctx.boxes_near(ctx.line_boxes[i], reach(c)) {
            let id = s.shape.id.as_str();
            let attached = own.contains(&id);
            if !attached
                && l.pieces
                    .iter()
                    .any(|&(p, q, _)| !inside_runs(p, q, &s.poly).is_empty())
            {
                continue;
            }
            let mut best: Option<(f64, usize, Pt, Pt)> = None;
            for &(p, q, k) in &l.pieces {
                for (e0, e1) in edges(&s.poly) {
                    let gap = if attached {
                        parallel(p, q, e0, e1).map(|(g, _)| g)
                    } else {
                        Some(seg_seg_dist(p, q, e0, e1))
                    };
                    if let Some(g) = gap.filter(|g| *g < c) {
                        if best.is_none_or(|b| g < b.0) {
                            best = Some((g, k, p, q));
                        }
                    }
                }
            }
            let Some((gap, k, p, q)) = best else { continue };
            if let Some((message, help)) = attached.then(|| wrong_elbow(ctx, l, s, p, q)).flatten()
            {
                let d = Diagnostic::warn(code::NEAR, l.line.span, message)
                    .with_element(line_el(l.line, Some(l.segment(k))))
                    .with_element(shape_el(s.shape))
                    .with_fact("distance", round2(gap))
                    .with_fact("clearance", c)
                    .with_help(help);
                out.push(d);
                continue;
            }
            let message = if attached {
                format!(
                    "{} runs along the edge of its own {} '{id}', {}px away (clearance {})",
                    l.line.id,
                    kind_name(s.shape),
                    fmt_num(gap),
                    fmt_num(c)
                )
            } else {
                format!(
                    "{} runs {}px from {} '{id}' (clearance {})",
                    l.line.id,
                    fmt_num(gap),
                    kind_name(s.shape),
                    fmt_num(c)
                )
            };
            let d = Diagnostic::warn(code::NEAR, l.line.span, message)
                .with_element(line_el(l.line, Some(l.segment(k))))
                .with_element(shape_el(s.shape))
                .with_fact("distance", round2(gap))
                .with_fact("clearance", c);
            let mut skip = vec![l.line.id.as_str()];
            skip.extend(own.iter().copied().filter(|o| *o != id));
            out.push(with_free(d, l, k, &skip));
        }
    }
}

/// A first or last piece that runs along the side of the box the line starts or ends on: the
/// elbow is the wrong way round (`hv` into a `left` side). The message and help to fix it.
fn wrong_elbow(ctx: &Ctx, l: &LineGeom, s: &ShapeGeom, p: Pt, q: Pt) -> Option<(String, String)> {
    let id = s.shape.id.as_str();
    let r = s.shape.rect;
    let vertical = (p.x - q.x).abs() <= TOLERANCE;
    if !vertical && (p.y - q.y).abs() > TOLERANCE {
        return None;
    }
    let side = match (vertical, (p.x - r.x).abs() <= (p.x - r.right()).abs()) {
        (true, true) => "left",
        (true, false) => "right",
        (false, _) if (p.y - r.y).abs() <= (p.y - r.bottom()).abs() => "top",
        (false, _) => "bottom",
    };
    let on = |pt: Pt| ctx.attached(pt).contains(&id);
    let (which, help) = if q.dist(l.end) <= TOLERANCE && on(l.end) {
        let help = if vertical {
            "enter a `left`/`right` side horizontally: finish with `vh` (or an `x …` hop, then the anchor)"
        } else {
            "enter a `top`/`bottom` side vertically: finish with `hv` (or a `y …` hop, then the anchor)"
        };
        ("last", help)
    } else if p.dist(l.start) <= TOLERANCE && on(l.start) {
        let help = if vertical {
            "leave a `left`/`right` side horizontally: start with `hv` (or an `x …` hop)"
        } else {
            "leave a `top`/`bottom` side vertically: start with `vh` (or a `y …` hop)"
        };
        ("first", help)
    } else {
        return None;
    };
    Some((
        format!(
            "{}'s {which} segment runs along the {side} side of its own {} '{id}'",
            l.line.id,
            kind_name(s.shape)
        ),
        help.into(),
    ))
}

/// Pieces that run parallel to a group's border within `clearance`, on it included. Pieces
/// that cross the border are not parallel to it, so they don't count.
fn near_groups(ctx: &Ctx, out: &mut Sink) {
    let c = ctx.scene.clearance;
    for (i, l) in ctx.lines.iter().enumerate() {
        for g in ctx.groups_near(ctx.line_boxes[i], reach(c)) {
            let mut best: Option<(f64, usize)> = None;
            for &(p, q, k) in &l.pieces {
                for (e0, e1) in edges(&g.poly) {
                    if let Some((gap, _)) = parallel(p, q, e0, e1).filter(|(g, _)| *g < c) {
                        if best.is_none_or(|b| gap < b.0) {
                            best = Some((gap, k));
                        }
                    }
                }
            }
            let Some((gap, k)) = best else { continue };
            let id = g.shape.id.as_str();
            let message = if gap <= TOLERANCE {
                format!(
                    "{} runs along the border of group '{id}' (clearance {})",
                    l.line.id,
                    fmt_num(c)
                )
            } else {
                format!(
                    "{} runs {}px from the border of group '{id}' (clearance {})",
                    l.line.id,
                    fmt_num(gap),
                    fmt_num(c)
                )
            };
            let d = Diagnostic::warn(code::NEAR, l.line.span, message)
                .with_element(line_el(l.line, Some(l.segment(k))))
                .with_element(shape_el(g.shape))
                .with_fact("distance", round2(gap))
                .with_fact("clearance", c);
            out.push(with_free(d, l, k, &[l.line.id.as_str()]));
        }
    }
}

fn near_lines(ctx: &Ctx, out: &mut Sink) {
    let c = ctx.scene.clearance;
    for (i, a) in ctx.lines.iter().enumerate() {
        for j in ctx.lines_near(ctx.line_boxes[i], reach(c)) {
            if j <= i {
                continue;
            }
            let b = &ctx.lines[j];
            let (mut run, mut gap, mut at) = (0.0, f64::INFINITY, None);
            for &(p, q, ai) in &a.pieces {
                let mut longest = 0.0f64;
                for &(e0, e1, bi) in &b.pieces {
                    let Some((g, len)) = parallel(p, q, e0, e1) else {
                        continue;
                    };
                    if g > TOLERANCE && g < c {
                        longest = longest.max(len);
                        gap = gap.min(g);
                        at.get_or_insert((ai, bi));
                    }
                }
                run += longest;
            }
            let Some((ai, bi)) = at else { continue };
            if run <= MIN_RUN {
                continue;
            }
            let d = Diagnostic::warn(
                code::NEAR,
                b.line.span,
                format!(
                    "{} runs {}px from {} for {}px (clearance {})",
                    b.line.id,
                    fmt_num(gap),
                    a.line.id,
                    fmt_num(run),
                    fmt_num(c)
                ),
            )
            .with_element(line_el(b.line, Some(b.segment(bi))))
            .with_element(line_el(a.line, Some(a.segment(ai))))
            .with_fact("distance", round2(gap))
            .with_fact("length", round2(run))
            .with_fact("clearance", c);
            out.push(with_free(d, b, bi, &[b.line.id.as_str()]));
        }
    }
}
