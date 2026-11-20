use kashika_diag::Diagnostic;
use kashika_geom::fmt_num;
use kashika_model::{TextItem, TextRole};

use crate::ctx::Ctx;
use crate::element::{bbox, shape_el};
use crate::sink::Sink;

fn describe(t: &TextItem) -> String {
    match t.role {
        TextRole::Label => "the label".to_string(),
        TextRole::Sublabel => "the sublabel".to_string(),
        TextRole::Stacked => format!("the stacked item \"{}\"", t.content),
        TextRole::Absolute => format!("block item \"{}\"", t.content),
    }
}

/// `text-overlap`: an absolute block item (`at` in the box-local frame) overlaps another text
/// of the same shape: its label, its sublabel, a stacked item or another absolute item.
pub fn text_overlap(ctx: &Ctx, out: &mut Sink) {
    for s in &ctx.scene.shapes {
        for (i, a) in s.text.iter().enumerate() {
            if a.role != TextRole::Absolute {
                continue;
            }
            for (j, b) in s.text.iter().enumerate() {
                if j == i || (b.role == TextRole::Absolute && j > i) {
                    continue;
                }
                let Some(o) = a.rect.intersect(b.rect) else {
                    continue;
                };
                out.push(
                    Diagnostic::warn(
                        "text-overlap",
                        a.span,
                        format!(
                            "block item \"{}\" overlaps {} of {} '{}' by {}×{}px",
                            a.content,
                            describe(b),
                            s.kind.name(),
                            s.id,
                            fmt_num(o.w),
                            fmt_num(o.h)
                        ),
                    )
                    .with_element(shape_el(s))
                    .with_fact("bbox", bbox(a.rect))
                    .with_fact("other", bbox(b.rect))
                    .with_help(match b.role {
                        TextRole::Label | TextRole::Sublabel => {
                            "move it with `at`, or drop the label and put it in the block"
                                .to_string()
                        }
                        _ => "move one of them with `at`".to_string(),
                    }),
                );
            }
        }
    }
}
