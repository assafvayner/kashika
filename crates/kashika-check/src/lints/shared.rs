use kashika_diag::Diagnostic;
use kashika_geom::{seg_hit, Hit, Pt, TOLERANCE};

use crate::ctx::{arr, fmt_num, fmt_pt, line_el, round2, Ctx, LineGeom};
use crate::free::free_around;

/// `shared-segment`: two lines overlap collinearly for more than 1px. Only straight pieces
/// are compared, because flattened curves would report artifacts.
pub fn shared_segment(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
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
                out.push(report(ctx, b, bi, a, ai, (p, q), total));
            }
        }
    }
    out
}

fn report(
    ctx: &Ctx,
    l: &LineGeom,
    li: usize,
    other: &LineGeom,
    oi: usize,
    (p, q): (Pt, Pt),
    total: f64,
) -> Diagnostic {
    let id = l.line.id.as_str();
    let mut d = Diagnostic::warn(
        "shared-segment",
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
    if let Some(f) = free_around(ctx, p, q, &[id]) {
        if let Some(lane) = f.nearest() {
            d = d
                .with_fact(
                    "lane",
                    serde_json::json!({"axis": f.axis.name(), "at": lane}),
                )
                .with_help(format!(
                    "nearest free lane: {}={}",
                    f.axis.name(),
                    fmt_num(lane)
                ));
        }
        d = d.with_fact("free", f.fact()).with_help(f.describe());
    }
    d
}
