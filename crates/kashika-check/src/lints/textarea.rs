use kashika_diag::Diagnostic;
use kashika_geom::{Rect, TOLERANCE, fmt_num};
use kashika_model::TextRole;

use crate::ctx::{Ctx, describe_overflow, overflow, overflow_fact};
use crate::element::{bbox, shape_el};
use crate::sink::Sink;

/// `text-outside-shape`: an absolute block item (`at` in the box-local frame) lies outside
/// its shape's text area, or the band under a `text-area=below` shape.
pub fn text_outside_shape(ctx: &Ctx, out: &mut Sink) {
    for s in &ctx.scene.shapes {
        let area = s.text_area;
        for item in s.text.iter().filter(|t| t.role == TextRole::Absolute) {
            let over = overflow(area, item.rect);
            if over.is_empty() {
                continue;
            }
            let local =
                |x: f64, y: f64| format!("({},{})", fmt_num(x - s.rect.x), fmt_num(y - s.rect.y));
            out.push(
                Diagnostic::warn(
                    "text-outside-shape",
                    item.span,
                    format!(
                        "block item \"{}\" lies outside the text area of {} '{}' ({})",
                        item.content,
                        s.kind.name(),
                        s.id,
                        describe_overflow(&over)
                    ),
                )
                .with_element(shape_el(s))
                .with_fact("bbox", bbox(item.rect))
                .with_fact("text_area", bbox(area))
                .with_fact("overflow", overflow_fact(&over))
                .with_help(shift_help(&over, item.rect, area))
                .with_help(format!(
                    "the text area is local {}..{}",
                    local(area.x, area.y),
                    local(area.right(), area.bottom())
                )),
            );
        }
    }
}

/// The `at` change that brings the item inside, or why none can.
fn shift_help(over: &[(&str, f64)], item: Rect, area: Rect) -> String {
    let by = |side: &str| over.iter().find(|(s, _)| *s == side).map(|(_, d)| *d);
    if item.w > area.w + TOLERANCE || item.h > area.h + TOLERANCE {
        return "the item is bigger than the text area: shorten it, break it with `\\n`, or enlarge the box".into();
    }
    let dx = by("left").unwrap_or_else(|| -by("right").unwrap_or(0.0));
    let dy = by("top").unwrap_or_else(|| -by("bottom").unwrap_or(0.0));
    let whole = |d: f64| if d < 0.0 { d.floor() } else { d.ceil() };
    format!(
        "add ({},{}) to its `at`",
        fmt_num(whole(dx)),
        fmt_num(whole(dy))
    )
}
