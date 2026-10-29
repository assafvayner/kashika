use kashika_diag::{Diagnostic, code};
use kashika_geom::{Hit, Pt, TOLERANCE, fmt_num, round2, seg_hit};

use crate::ctx::{Ctx, LineGeom, arr, fmt_pt, line_el};
use crate::free::{DeferredKind, with_deferred};
use crate::sink::Sink;

/// `shared-segment`: two lines overlap collinearly for more than 1px. Only straight pieces
/// are compared, because flattened curves would report artifacts.
pub fn shared_segment(ctx: &Ctx, out: &mut Sink) {
    for (i, a) in ctx.lines.iter().enumerate() {
        for j in ctx.lines_near(ctx.line_boxes[i], TOLERANCE) {
            if j <= i {
                continue;
            }
            let b = &ctx.lines[j];
            let mut total = 0.0;
            let mut first: Option<(Pt, Pt, usize, usize)> = None;
            for &(a0, a1, ai) in a.pieces.iter().filter(|p| a.straight(p.2)) {
                for &(b0, b1, bi) in b.pieces.iter().filter(|p| b.straight(p.2)) {
                    if let Some(Hit::Overlap(p, q)) = seg_hit(a0, a1, b0, b1, TOLERANCE) {
                        if p.dist(q) > 1.0 {
                            total += p.dist(q);
                            first.get_or_insert((p, q, ai, bi));
                        }
                    }
                }
            }
            if let Some((p, q, ai, bi)) = first {
                out.push(report(b, bi, a, ai, (p, q), total));
            }
        }
    }
}

fn report(
    l: &LineGeom,
    li: usize,
    other: &LineGeom,
    oi: usize,
    (p, q): (Pt, Pt),
    total: f64,
) -> Diagnostic {
    let id = l.line.id.as_str();
    let d = Diagnostic::warn(
        code::SHARED_SEGMENT,
        l.line.span,
        format!(
            "{id} runs along {} for {}px, from {} to {}",
            other.line.id,
            fmt_num(total),
            fmt_pt(p),
            fmt_pt(q)
        ),
    )
    .with_element(line_el(l.line, Some(l.segment(li))))
    .with_element(line_el(other.line, Some(other.segment(oi))))
    .with_fact("from", arr(p))
    .with_fact("to", arr(q))
    .with_fact("length", round2(total));
    let d = with_deferred(d, DeferredKind::Lane, p, q, &[id]);
    let shared_end = [(l.start, other.start), (l.end, other.end)]
        .into_iter()
        .any(|(a, b)| a.dist(b) <= TOLERANCE);
    if shared_end {
        d.with_help(
            "both lines leave or reach the same point: spread them along the side with `side@t` (`a.right@0.3`, `a.right@0.7`), or fan out from a `junction`",
        )
    } else {
        d
    }
}
