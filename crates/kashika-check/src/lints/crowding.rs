use std::collections::HashMap;

use kashika_diag::Diagnostic;
use kashika_geom::TOLERANCE;

use crate::ctx::{fmt_num, fmt_pt, line_el, round2, shape_el, Ctx, LineGeom};
use crate::ports::ports;

const MIN_GAP: f64 = 8.0;

/// `port-crowding`: two endpoints on one shape are within 8px, or are identical and both
/// draw a marker (stacked arrowheads). Junction endpoints are exempt.
pub fn port_crowding(ctx: &Ctx) -> Vec<Diagnostic> {
    let all = ports(ctx);
    let mut by_shape: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, p) in all.iter().enumerate() {
        by_shape.entry(p.shape.as_str()).or_default().push(i);
    }
    let mut out = Vec::new();
    for (i, a) in all.iter().enumerate() {
        for b in by_shape[a.shape.as_str()]
            .iter()
            .filter(|&&j| j > i)
            .map(|&j| &all[j])
        {
            let gap = a.at.dist(b.at);
            let stacked = gap <= TOLERANCE;
            if gap >= MIN_GAP || (stacked && !(a.marker && b.marker)) || a.line == b.line {
                continue;
            }
            let (Some(la), Some(lb), Some(s)) =
                (ctx.line(&a.line), ctx.line(&b.line), ctx.shape(&a.shape))
            else {
                continue;
            };
            let message = if stacked {
                format!(
                    "{} and {} both draw a marker at {} on '{}'",
                    b.line,
                    a.line,
                    fmt_pt(b.at),
                    s.id
                )
            } else {
                format!(
                    "{} and {} meet '{}' {}px apart (minimum 8px)",
                    b.line,
                    a.line,
                    s.id,
                    fmt_num(gap)
                )
            };
            out.push(
                Diagnostic::warn("port-crowding", lb.line.span, message)
                    .with_element(line_el(lb.line, Some(end_segment(lb, b.end))))
                    .with_element(line_el(la.line, Some(end_segment(la, a.end))))
                    .with_element(shape_el(s))
                    .with_fact("gap", round2(gap))
                    .with_help(format!(
                        "spread the endpoints at least 8px apart, e.g. `{0}.{1}@0.3` and `{0}.{1}@0.7`, or meet at a `junction`",
                        s.id, b.side
                    )),
            );
        }
    }
    out
}

fn end_segment(l: &LineGeom, end: &str) -> usize {
    if end == "start" {
        0
    } else {
        l.last_segment()
    }
}
