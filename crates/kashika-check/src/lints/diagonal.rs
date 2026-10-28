use kashika_diag::Diagnostic;
use kashika_geom::PathSeg;

use crate::ctx::{fmt_pt, line_el, Ctx};

/// `diagonal-tail`: the final segment is diagonal while every other straight segment is
/// horizontal or vertical. A line that is a single diagonal has no tail.
pub fn diagonal_tail(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for l in &ctx.lines {
        let path = &l.line.path;
        let straight: Vec<usize> = (0..path.len())
            .filter(|&i| matches!(path[i], PathSeg::Line { .. }))
            .collect();
        let Some(&last) = straight.last() else {
            continue;
        };
        if last != path.len() - 1
            || straight.len() < 2
            || path[last].is_axis_aligned()
            || !straight[..straight.len() - 1]
                .iter()
                .all(|&i| path[i].is_axis_aligned())
        {
            continue;
        }
        let seg = l.segment(last);
        let span = l.line.segments.get(seg).map_or(l.line.span, |s| s.span);
        out.push(
            Diagnostic::warn(
                "diagonal-tail",
                span,
                format!(
                    "the last segment of {} runs diagonally from {} to {}",
                    l.line.id,
                    fmt_pt(path[last].from()),
                    fmt_pt(path[last].to())
                ),
            )
            .with_element(line_el(l.line, Some(seg)))
            .with_help("finish with `hv`/`vh` or an `x`/`y` target so the last hop is horizontal or vertical"),
        );
    }
    out
}
