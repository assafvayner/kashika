use kashika_diag::Diagnostic;
use kashika_geom::{Rect, TOLERANCE};
use kashika_model::{Shape, ShapeKind, TextItem, TextRole};

use crate::ctx::{Ctx, kind_name, shape_el};

/// Size of a `sublabel` relative to the label.
const SUBLABEL_SCALE: f64 = 0.8;

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

/// `block-reducible`: a text block of 1–2 stacked items that a `label`, optionally with a
/// `sublabel` or a second line, would draw the same way.
pub fn block_reducible(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for s in &ctx.scene.shapes {
        let Some(rewrite) = rewrite(s) else { continue };
        let kind = kind_name(s);
        let span = s.text.first().map_or(s.span, |t| t.span);
        out.push(
            Diagnostic::warn(
                "block-reducible",
                span,
                format!(
                    "the text block of {kind} '{}' can be written as label/sublabel",
                    s.id
                ),
            )
            .with_element(shape_el(s))
            .with_fact("rewrite", &rewrite)
            .with_help(format!(
                "drop the block and write `{kind} {} {rewrite}`",
                s.id
            )),
        );
    }
    out
}

/// The `"label" [sublabel="..."] [text-align=...]` text that replaces `s`'s text block.
fn rewrite(s: &Shape) -> Option<String> {
    if s.text.iter().any(|t| t.role != TextRole::Stacked) {
        return None;
    }
    let (first, second) = match s.text.as_slice() {
        [a] => (a, None),
        [a, b] => (a, Some(b)),
        _ => return None,
    };
    let st = &s.style;
    let as_label = |t: &TextItem| t.size == st.font_size && t.color == st.color;
    if !as_label(first) {
        return None;
    }
    let mut text = quote(&first.content);
    match second {
        None => {}
        Some(b) if as_label(b) => {
            text = quote(&format!("{}\n{}", first.content, b.content));
        }
        Some(b)
            if (b.size - st.font_size * SUBLABEL_SCALE).abs() < 1e-9
                && b.color == st.muted
                && !b.bold =>
        {
            text = format!("{text} sublabel={}", quote(&b.content));
        }
        Some(_) => return None,
    }
    let align = align(s)?;
    let default = if s.kind == ShapeKind::Group {
        Align::Left
    } else {
        Align::Center
    };
    if align != default {
        let name = match align {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        };
        text = format!("{text} text-align={name}");
    }
    Some(text)
}

/// The one alignment shared by every stacked item, preferring the shape's default.
fn align(s: &Shape) -> Option<Align> {
    let area = s.text_area;
    let fits = |a: Align, r: Rect| match a {
        Align::Left => same(r.x, area.x),
        Align::Center => same(r.x + r.w / 2.0, area.x + area.w / 2.0),
        Align::Right => same(r.right(), area.right()),
    };
    let order = if s.kind == ShapeKind::Group {
        [Align::Left, Align::Center, Align::Right]
    } else {
        [Align::Center, Align::Left, Align::Right]
    };
    order
        .into_iter()
        .find(|&a| s.text.iter().all(|t| fits(a, t.rect)))
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE
}

/// A Kashika string literal for text whose markup escapes are already in source form.
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\\\"").replace('\n', "\\n"))
}
