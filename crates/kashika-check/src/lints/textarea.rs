use kashika_diag::Diagnostic;
use kashika_model::TextRole;

use crate::ctx::{
    bbox, describe_overflow, fmt_num, kind_name, overflow, overflow_fact, shape_el, Ctx,
};

/// `text-outside-shape`: an absolute block item (`at` in the box-local frame) lies outside
/// its shape's text area, or the band under a `text-area=below` shape.
pub fn text_outside_shape(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
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
                        kind_name(s),
                        s.id,
                        describe_overflow(&over)
                    ),
                )
                .with_element(shape_el(s))
                .with_fact("bbox", bbox(item.rect))
                .with_fact("text_area", bbox(area))
                .with_fact("overflow", overflow_fact(&over))
                .with_help(format!(
                    "keep the item within the text area, local {}..{}",
                    local(area.x, area.y),
                    local(area.right(), area.bottom())
                )),
            );
        }
    }
    out
}
