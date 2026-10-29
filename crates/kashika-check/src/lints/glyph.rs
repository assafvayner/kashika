use kashika_diag::{Diagnostic, Element, Span};
use kashika_model::PlacedLine;
use kashika_text::{covered_scripts, missing_glyphs};

use crate::ctx::Ctx;
use crate::element::{line_el, shape_el, text_el, text_id};
use crate::sink::Sink;

/// `missing-glyph`: text uses characters the bundled fonts cannot draw. They render as the
/// font's empty-box `.notdef` glyph.
pub fn missing_glyph(ctx: &Ctx, out: &mut Sink) {
    for s in &ctx.shapes {
        for item in &s.shape.text {
            check(
                out,
                &item.lines,
                item.span,
                format!("text of '{}'", s.shape.id),
                shape_el(s.shape),
            );
        }
    }
    for t in &ctx.scene.texts {
        check(
            out,
            &t.lines,
            t.span,
            format!("text '{}'", text_id(t)),
            text_el(t),
        );
    }
    for l in &ctx.lines {
        for lab in &l.line.labels {
            check(
                out,
                &lab.lines,
                lab.span,
                format!("label of {}", l.line.id),
                line_el(l.line, None),
            );
        }
    }
}

fn check(out: &mut Sink, lines: &[PlacedLine], span: Span, what: String, el: Element) {
    let missing = missing_glyphs(lines.iter().flat_map(|l| &l.runs));
    if missing.is_empty() {
        return;
    }
    let list = missing
        .iter()
        .map(|c| format!("U+{:04X}", u32::from(*c)))
        .collect::<Vec<_>>();
    out.push(
        Diagnostic::warn(
            "missing-glyph",
            span,
            format!(
                "{what} uses {} the bundled fonts cannot draw: {}",
                if missing.len() == 1 {
                    "a character"
                } else {
                    "characters"
                },
                missing
                    .iter()
                    .map(|c| format!("'{}'", c.escape_debug()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_element(el)
        .with_fact("codepoints", list)
        .with_help(format!(
            "they draw as an empty box; use characters from {}, or an `icon=`/`image`",
            scripts()
        )),
    );
}

/// The scripts the bundled fonts cover, as `A, B or C`.
fn scripts() -> String {
    match covered_scripts().as_slice() {
        [] => "the bundled fonts".to_string(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}
