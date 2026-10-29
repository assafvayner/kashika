use kashika_diag::Diagnostic;
use kashika_geom::{Rect, TOLERANCE, boundary_nearest, fmt_num};

use crate::ctx::{Ctx, anchor_text, fmt_pt};
use crate::element::{arr, line_el, shape_el};
use crate::sink::Sink;

/// How close a literal endpoint must be to an outline to look like a missed anchor.
const SNAP: f64 = 8.0;

/// `endpoint-off-edge`: a literal endpoint is within 8px of a shape outline but not on it.
pub fn endpoint_off_edge(ctx: &Ctx, out: &mut Sink) {
    for l in &ctx.lines {
        let ends = [
            (l.start, l.line.start_literal, "starts", 0),
            (l.end, l.line.end_literal, "ends", l.last_segment()),
        ];
        for (p, literal, verb, seg) in ends {
            if !literal {
                continue;
            }
            let best = ctx
                .shapes_near(Rect::new(p.x, p.y, 0.0, 0.0), SNAP)
                .into_iter()
                .map(|i| &ctx.shapes[i])
                .filter(|s| s.shape.layer.is_normal())
                .map(|s| {
                    let (d, q) = boundary_nearest(p, &s.poly);
                    (d, q, s)
                })
                .filter(|(d, _, _)| *d > TOLERANCE && *d <= SNAP)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let Some((d, q, s)) = best else { continue };
            let span = if seg == 0 && verb == "starts" {
                l.line.span
            } else {
                l.line.segments.get(seg).map_or(l.line.span, |g| g.span)
            };
            out.push(
                Diagnostic::warn(
                    "endpoint-off-edge",
                    span,
                    format!(
                        "{} {verb} at {}, {}px from the outline of '{}'",
                        l.line.id,
                        fmt_pt(p),
                        fmt_num(d),
                        s.shape.id
                    ),
                )
                .with_element(line_el(l.line, Some(seg)))
                .with_element(shape_el(s.shape))
                .with_fact("nearest", arr(q))
                .with_help(format!(
                    "snap it to the outline with an anchor: `{}`",
                    anchor_text(s.shape, q)
                )),
            );
        }
    }
}
