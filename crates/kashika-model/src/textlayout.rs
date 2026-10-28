use kashika_diag::{Diagnostic, Span};
use kashika_geom::{Pt, Rect, TOLERANCE};
use kashika_syntax::ast;
use kashika_text::{measure, stack, stack_size, Block, HAlign, MarkupError, PlacedLine, VAlign};
use serde::Serialize;

use crate::attrs::{self, Attrs, LIMIT};
use crate::scene::{Align, ShapeKind};
use crate::shapes;
use crate::style::{FontWeight, Style};

/// Attributes a box or group accepts for its text, besides the style keys.
pub(crate) const SHAPE_TEXT_KEYS: &[&str] = &["sublabel", "text-align", "text-valign", "gap"];
const BLOCK_ITEM_KEYS: &[&str] = &["size", "color", "align", "muted"];
const SUBLABEL_SCALE: f64 = 0.8;
const DEFAULT_GAP: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextRole {
    Label,
    Sublabel,
    Stacked,
    Absolute,
}

/// One laid-out piece of text inside a box or group.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextItem {
    pub role: TextRole,
    /// The source text, with markup.
    pub content: String,
    pub size: f64,
    pub color: String,
    pub bold: bool,
    pub rect: Rect,
    pub lines: Vec<PlacedLine>,
    pub span: Span,
}

pub(crate) struct ShapeTextProps {
    sublabel: Option<(String, Span)>,
    align: Align,
    valign: VAlign,
    gap: f64,
}

pub(crate) fn shape_text_props(
    a: &Attrs,
    group: bool,
    diags: &mut Vec<Diagnostic>,
) -> ShapeTextProps {
    let sublabel = a
        .get("sublabel")
        .and_then(|x| attrs::text(x, diags).map(|s| (s, x.span)));
    let align = match a
        .get("text-align")
        .and_then(|x| attrs::word(x, &["left", "center", "right"], diags))
    {
        Some("left") => Align::Left,
        Some("right") => Align::Right,
        Some(_) => Align::Center,
        None if group => Align::Left,
        None => Align::Center,
    };
    let mut valign = if group { VAlign::Top } else { VAlign::Middle };
    if let Some(x) = a.get("text-valign") {
        if group {
            diags.push(
                Diagnostic::error(
                    "attr",
                    x.span,
                    "a group's title always sits at the top; `text-valign` applies to boxes only",
                )
                .with_help("remove `text-valign`"),
            );
        } else {
            valign = match attrs::word(x, &["top", "middle", "bottom"], diags) {
                Some("top") => VAlign::Top,
                Some("bottom") => VAlign::Bottom,
                _ => VAlign::Middle,
            };
        }
    }
    let mut gap = DEFAULT_GAP;
    if let Some(x) = a.get("gap") {
        if let Some(v) = attrs::num(x, diags) {
            if v < 0.0 {
                diags.push(Diagnostic::error(
                    "attr",
                    x.span,
                    "`gap` must not be negative",
                ));
            } else {
                gap = v;
            }
        }
    }
    ShapeTextProps {
        sublabel,
        align,
        valign,
        gap,
    }
}

/// A shape's text area (§6): the shape's own text area (the whole box for `rect` and
/// `rounded`) inset by `padding`.
pub(crate) fn text_area(shape: &str, custom: Option<Rect>, rect: Rect, padding: f64) -> Rect {
    let unit = custom.or_else(|| shapes::unit_text_area(shape, rect.w, rect.h));
    let r = unit.map_or(rect, |u| rect.map_rect(u));
    Rect::new(
        r.x + padding,
        r.y + padding,
        (r.w - 2.0 * padding).max(0.0),
        (r.h - 2.0 * padding).max(0.0),
    )
}

/// The smallest box size, at least `rect`'s, whose text area holds `need` (width, height).
fn required_size(
    shape: &str,
    custom: Option<Rect>,
    need: (f64, f64),
    padding: f64,
    rect: Rect,
) -> (f64, f64) {
    let (w, h) = shapes::required_size(
        shape,
        custom,
        (need.0 + 2.0 * padding, need.1 + 2.0 * padding),
        rect,
    );
    (w.max(rect.w), h.max(rect.h))
}

pub(crate) fn halign(a: Align) -> HAlign {
    match a {
        Align::Left => HAlign::Left,
        Align::Center => HAlign::Center,
        Align::Right => HAlign::Right,
    }
}

/// Measures `text`, reporting unclosed markup as a `parse` error at `span`.
pub(crate) fn measure_at(
    text: &str,
    size: f64,
    bold: bool,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) -> Option<Block> {
    measure(text, size, bold)
        .map_err(|e: MarkupError| {
            diags.push(
                Diagnostic::error("parse", span, e.to_string())
                    .with_help("close the marker, or write `\\*` or `\\`` for a literal one"),
            );
        })
        .ok()
}

/// A text `size=`: greater than 0 and at most `LIMIT`, else a `size` error.
pub(crate) fn text_size(x: &ast::Attr, diags: &mut Vec<Diagnostic>) -> Option<f64> {
    let v = attrs::num(x, diags)?;
    if v <= 0.0 || v > LIMIT {
        diags.push(Diagnostic::error(
            "size",
            x.span,
            format!("text size must be greater than 0 and at most {LIMIT}"),
        ));
        return None;
    }
    Some(v)
}

/// A text `color=`, which can't be `none`.
pub(crate) fn text_color(x: &ast::Attr, diags: &mut Vec<Diagnostic>) -> Option<String> {
    let c = attrs::color(x, diags)?;
    if c == "none" {
        diags.push(attrs::type_err(x, "a color like `#1a73e8`"));
        return None;
    }
    Some(c)
}

/// Rounds to 2 decimals for messages, like the SVG writer does.
fn px(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 {
        "0".into()
    } else {
        format!("{r}")
    }
}

pub(crate) struct ShapeText {
    pub area: Rect,
    pub items: Vec<TextItem>,
    pub title_band: f64,
}

pub(crate) struct ShapeInput<'x> {
    pub id: &'x str,
    pub node: &'x ast::Node,
    pub kind: ShapeKind,
    /// The `shape` name, `rect` or `rounded` for groups.
    pub shape: &'x str,
    /// Unit text area of a custom shape; `None` for built-in shapes.
    pub custom_text_area: Option<Rect>,
    /// `text-area=below`: the text sits in a band `padding` under the box.
    pub text_below: bool,
    /// How far an icon pushes the text area from the left and from the top edge.
    pub icon_inset: (f64, f64),
    pub rect: Rect,
    pub style: &'x Style,
    pub props: &'x ShapeTextProps,
    /// Canvas position of each block item's `at`, in block order (`None` for stacked items
    /// and for points that failed to resolve).
    pub block_at: &'x [Option<Pt>],
}

struct Pending {
    role: TextRole,
    content: String,
    block: Block,
    color: String,
    bold: bool,
    align: Align,
    span: Span,
}

/// Lays out a shape's label, sublabel and text block, and reports `text-conflict`,
/// `label-overflow` and markup errors.
pub(crate) fn layout_shape(s: &ShapeInput, diags: &mut Vec<Diagnostic>) -> ShapeText {
    let st = s.style;
    let p = s.props;
    let mut area = text_area(s.shape, s.custom_text_area, s.rect, st.padding);
    let bold = st.font_weight == FontWeight::Bold;
    let mut head: Vec<Pending> = Vec::new();
    if let Some(label) = &s.node.label {
        if let Some(block) = measure_at(label, st.font_size, bold, s.node.span, diags) {
            head.push(Pending {
                role: TextRole::Label,
                content: label.clone(),
                block,
                color: st.color.clone(),
                bold,
                align: p.align,
                span: s.node.span,
            });
        }
    }
    if let Some((sub, span)) = &p.sublabel {
        if let Some(block) = measure_at(sub, st.font_size * SUBLABEL_SCALE, false, *span, diags) {
            head.push(Pending {
                role: TextRole::Sublabel,
                content: sub.clone(),
                block,
                color: st.muted.clone(),
                bold: false,
                align: p.align,
                span: *span,
            });
        }
    }

    let mut stacked: Vec<Pending> = Vec::new();
    let mut absolute: Vec<(Pending, Pt)> = Vec::new();
    for (i, item) in s.node.block.iter().flatten().enumerate() {
        let Some(pending) = block_item(item, st, p.align, diags) else {
            continue;
        };
        match (&item.at, s.block_at.get(i).copied().flatten()) {
            (None, _) => stacked.push(pending),
            (Some(_), Some(at)) => absolute.push((pending, at)),
            (Some(_), None) => {}
        }
    }
    let has_head = s.node.label.is_some() || p.sublabel.is_some();
    if has_head && !stacked.is_empty() {
        diags.push(
            Diagnostic::error(
                "text-conflict",
                stacked[0].span,
                format!(
                    "`{}` has a label or sublabel and stacked text block items",
                    s.id
                ),
            )
            .with_help("move the label into the block as its first `text` item, or give every block item its own `at`"),
        );
    }
    let flow = if has_head { head } else { stacked };
    if s.text_below {
        area = below_area(s.rect, &flow, st, p.gap);
    } else {
        let (dx, dy) = s.icon_inset;
        area = Rect::new(
            area.x + dx,
            area.y + dy,
            (area.w - dx).max(0.0),
            (area.h - dy).max(0.0),
        );
    }

    let mut items = Vec::new();
    let mut title_band = 0.0;
    if !flow.is_empty() {
        let blocks: Vec<&Block> = flow.iter().map(|x| &x.block).collect();
        let need = stack_size(&blocks, p.gap);
        if need.0 > area.w + TOLERANCE || need.1 > area.h + TOLERANCE {
            diags.push(overflow(s, "its text", need, area, flow[0].span));
        }
        let input: Vec<(&Block, HAlign)> =
            flow.iter().map(|x| (&x.block, halign(x.align))).collect();
        for (x, (lines, rect)) in flow.iter().zip(stack(&input, area, p.valign, p.gap)) {
            items.push(item(x, rect, lines));
        }
        if s.kind == ShapeKind::Group {
            title_band = st.padding + need.1;
        }
    }
    for (x, at) in &absolute {
        let need = (x.block.width, x.block.height);
        if need.0 > area.w + TOLERANCE || need.1 > area.h + TOLERANCE {
            diags.push(overflow(s, "a block item", need, area, x.span));
        }
        let (lines, rect) = x.block.place_at(at.x, at.y, halign(x.align));
        items.push(item(x, rect, lines));
    }
    ShapeText {
        area,
        items,
        title_band,
    }
}

/// The band under a `text-area=below` shape: `padding` below the bottom edge, as tall as
/// the stacked text (one line when there is none) and as wide as the box or the text,
/// whichever is wider, centered on the box.
fn below_area(rect: Rect, flow: &[Pending], st: &Style, gap: f64) -> Rect {
    let blocks: Vec<&Block> = flow.iter().map(|x| &x.block).collect();
    let (w, h) = if blocks.is_empty() {
        (0.0, st.font_size * 1.25)
    } else {
        stack_size(&blocks, gap)
    };
    let w = w.max(rect.w);
    Rect::new(
        rect.x + (rect.w - w) / 2.0,
        rect.bottom() + st.padding,
        w,
        h,
    )
}

fn item(x: &Pending, rect: Rect, lines: Vec<PlacedLine>) -> TextItem {
    TextItem {
        role: x.role,
        content: x.content.clone(),
        size: x.block.size,
        color: x.color.clone(),
        bold: x.bold,
        rect,
        lines,
        span: x.span,
    }
}

fn block_item(
    item: &ast::BlockText,
    st: &Style,
    align: Align,
    diags: &mut Vec<Diagnostic>,
) -> Option<Pending> {
    let a = Attrs::new(&item.attrs, "text block item", BLOCK_ITEM_KEYS, diags);
    let size = a
        .get("size")
        .and_then(|x| text_size(x, diags))
        .unwrap_or(st.font_size);
    let muted = a.get("muted").is_some_and(|x| attrs::flag(x, diags));
    let color = a
        .get("color")
        .and_then(|x| text_color(x, diags))
        .unwrap_or_else(|| {
            if muted {
                st.muted.clone()
            } else {
                st.color.clone()
            }
        });
    let align = match a
        .get("align")
        .and_then(|x| attrs::word(x, &["left", "center", "right"], diags))
    {
        Some("left") => Align::Left,
        Some("right") => Align::Right,
        Some(_) => Align::Center,
        None => align,
    };
    let bold = st.font_weight == FontWeight::Bold;
    let block = measure_at(&item.content, size, bold, item.span, diags)?;
    Some(Pending {
        role: if item.at.is_some() {
            TextRole::Absolute
        } else {
            TextRole::Stacked
        },
        content: item.content.clone(),
        block,
        color,
        bold,
        align,
        span: item.span,
    })
}

fn overflow(s: &ShapeInput, what: &str, need: (f64, f64), area: Rect, span: Span) -> Diagnostic {
    let inset = if s.text_below {
        (0.0, 0.0)
    } else {
        s.icon_inset
    };
    let need = (need.0 + inset.0, need.1 + inset.1);
    let (w, h) = required_size(s.shape, s.custom_text_area, need, s.style.padding, s.rect);
    let size = (w.ceil(), h.ceil());
    Diagnostic::error(
        "label-overflow",
        span,
        format!(
            "{what} needs {}x{}px but the text area of `{}` is {}x{}px",
            px(need.0),
            px(need.1),
            s.id,
            px(area.w),
            px(area.h)
        ),
    )
    .with_help(format!(
        "make `{}` at least size ({},{}), or shorten the text",
        s.id, size.0, size.1
    ))
}
