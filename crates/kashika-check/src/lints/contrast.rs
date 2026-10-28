use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{Pt, Rect, point_in_polygon};
use kashika_model::{Shape, ShapeKind, TextRole};

use crate::ctx::{Ctx, line_el, round2, shape_el, text_el, text_id};

const MIN_RATIO: f64 = 3.0;
const WHITE: [f64; 3] = [1.0, 1.0, 1.0];

/// `low-contrast`: the WCAG contrast ratio between text and the fill behind it is below 3:1.
pub fn low_contrast(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for s in &ctx.shapes {
        let sh = s.shape;
        for item in &sh.text {
            let what = match item.role {
                TextRole::Label => format!("label \"{}\"", item.content),
                TextRole::Sublabel => format!("sublabel \"{}\"", item.content),
                TextRole::Stacked | TextRole::Absolute => format!("text \"{}\"", item.content),
            };
            let bg = backdrop(ctx, item.rect.center(), Some(&sh.id));
            check(
                &mut out,
                &item.color,
                bg,
                shape_el(sh),
                item.span,
                format!("{what} of '{}'", sh.id),
            );
        }
    }
    for t in &ctx.scene.texts {
        let bg = backdrop(ctx, t.at, None);
        check(
            &mut out,
            &t.color,
            bg,
            text_el(t),
            t.span,
            format!("text '{}'", text_id(t)),
        );
    }
    for l in &ctx.lines {
        for lab in &l.line.labels {
            let what = format!("label \"{}\" of {}", lab.text, l.line.id);
            let bg = backdrop(ctx, lab.rect.center(), None);
            check(
                &mut out,
                &l.line.style.color,
                bg,
                line_el(l.line, None),
                lab.span,
                what,
            );
        }
    }
    out
}

fn check(
    out: &mut Vec<Diagnostic>,
    color: &str,
    bg: [f64; 3],
    el: Element,
    span: Span,
    what: String,
) {
    let Some(fg) = rgba(color) else { return };
    let r = ratio(over(fg, bg), bg);
    if r >= MIN_RATIO {
        return;
    }
    out.push(
        Diagnostic::warn(
            "low-contrast",
            span,
            format!("{what} has contrast {r:.1}:1 against its background (minimum 3:1)"),
        )
        .with_element(el)
        .with_fact("ratio", round2(r))
        .with_fact("color", color)
        .with_fact("background", hex(bg))
        .with_help("darken the text `color` or lighten the `fill` behind it"),
    );
}

/// The canvas with every fill containing `p` painted on top in paint order (groups, then
/// boxes), including the shape `last` when given and none after it. A transparent canvas
/// counts as white. Images and SVG-file shapes draw their file instead of a fill, so they
/// add nothing.
fn backdrop(ctx: &Ctx, p: Pt, last: Option<&str>) -> [f64; 3] {
    let canvas = ctx.scene.background.as_deref().and_then(rgba);
    let mut c = canvas.map_or(WHITE, |bg| over(bg, WHITE));
    let order = |i: usize| (ctx.shapes[i].shape.kind != ShapeKind::Group, i);
    let stop = last.and_then(|id| ctx.shape_index(id)).map(order);
    let mut near = ctx.shapes_near(Rect::new(p.x, p.y, 0.0, 0.0), 0.0);
    near.sort_by_key(|&i| order(i));
    for i in near {
        if stop.is_some_and(|s| order(i) > s) {
            break;
        }
        let s = &ctx.shapes[i];
        if paints_fill(s.shape) && point_in_polygon(p, &s.poly) {
            if let Some(fill) = rgba(&s.shape.style.fill) {
                c = over(fill, c);
            }
        }
    }
    c
}

fn paints_fill(s: &Shape) -> bool {
    s.kind != ShapeKind::Image
        && s.image.is_none()
        && s.form.as_ref().is_none_or(|f| !f.draw.is_empty())
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` as 0..1 channels; `None` for `none` or anything else.
fn rgba(c: &str) -> Option<[f64; 4]> {
    let h = c.strip_prefix('#')?;
    let v: Vec<f64> = match h.len() {
        3 => h
            .chars()
            .map(|ch| ch.to_digit(16).map(|d| f64::from(d * 17)))
            .collect::<Option<_>>()?,
        6 | 8 => (0..h.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(f64::from))
            .collect::<Option<_>>()?,
        _ => return None,
    };
    Some([
        v[0] / 255.0,
        v[1] / 255.0,
        v[2] / 255.0,
        v.get(3).map_or(1.0, |a| a / 255.0),
    ])
}

fn over(top: [f64; 4], under: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| top[i] * top[3] + under[i] * (1.0 - top[3]))
}

fn luminance(c: [f64; 3]) -> f64 {
    let lin = |v: f64| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn hex(c: [f64; 3]) -> String {
    let b = |v: f64| (v * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}
