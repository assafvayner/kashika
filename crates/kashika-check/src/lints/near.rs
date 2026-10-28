use kashika_diag::Diagnostic;
use kashika_geom::{cross, dot, edges, inside_runs, seg_seg_dist, Pt, TOLERANCE};

use crate::ctx::{fmt_num, kind_name, line_el, round2, shape_el, Ctx};
use crate::free::with_free;

/// Pieces whose directions differ by less than ~3° count as parallel.
const PARALLEL: f64 = 0.05;
/// Parallel lines must run side by side for longer than this to be `near`.
const MIN_RUN: f64 = 20.0;

/// `near`: a line runs within `clearance` of a box edge, or of a parallel line for more than 20px.
pub fn near(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = near_boxes(ctx);
    out.extend(near_lines(ctx));
    out
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

fn near_boxes(ctx: &Ctx) -> Vec<Diagnostic> {
    let c = ctx.scene.clearance;
    let mut out = Vec::new();
    for l in &ctx.lines {
        let mut own = ctx.attached(l.start);
        own.extend(ctx.attached(l.end));
        for s in ctx.boxes() {
            let id = s.shape.id.as_str();
            let attached = own.contains(&id);
            if !attached
                && l.pieces
                    .iter()
                    .any(|&(p, q, _)| !inside_runs(p, q, &s.poly).is_empty())
            {
                continue;
            }
            let mut best: Option<(f64, usize)> = None;
            for &(p, q, k) in &l.pieces {
                for (e0, e1) in edges(&s.poly) {
                    let gap = if attached {
                        parallel(p, q, e0, e1).map(|(g, _)| g)
                    } else {
                        Some(seg_seg_dist(p, q, e0, e1))
                    };
                    if let Some(g) = gap.filter(|g| *g < c) {
                        if best.is_none_or(|b| g < b.0) {
                            best = Some((g, k));
                        }
                    }
                }
            }
            let Some((gap, k)) = best else { continue };
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
            let d = Diagnostic::warn("near", l.line.span, message)
                .with_element(line_el(l.line, Some(l.segment(k))))
                .with_element(shape_el(s.shape))
                .with_fact("distance", round2(gap))
                .with_fact("clearance", c);
            let mut skip = vec![l.line.id.as_str()];
            skip.extend(own.iter().copied().filter(|o| *o != id));
            out.push(with_free(ctx, d, l, k, &skip));
        }
    }
    out
}

fn near_lines(ctx: &Ctx) -> Vec<Diagnostic> {
    let c = ctx.scene.clearance;
    let mut out = Vec::new();
    for (i, a) in ctx.lines.iter().enumerate() {
        for b in &ctx.lines[i + 1..] {
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
                "near",
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
            out.push(with_free(ctx, d, b, bi, &[b.line.id.as_str()]));
        }
    }
    out
}
