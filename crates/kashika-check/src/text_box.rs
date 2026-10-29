use kashika_geom::{Pt, Rect};
use kashika_model::{Label, Shape, ShapeKind, Text, TextRole};

pub fn text_box(t: &Text) -> Rect {
    t.rect
}

pub fn label_box(label: &Label) -> Rect {
    label.rect
}

/// The box `label` would occupy if it were centered on `at` instead.
pub fn label_box_at(label: &Label, at: Pt) -> Rect {
    let r = label.rect;
    Rect::new(r.x + at.x - label.at.x, r.y + at.y - label.at.y, r.w, r.h)
}

/// The label of a `text-area=below` shape, which sits under the shape instead of inside it.
pub fn below_box(s: &Shape) -> Option<Rect> {
    if !s.text_below {
        return None;
    }
    s.text
        .iter()
        .filter(|t| t.role != TextRole::Absolute)
        .map(|t| t.rect)
        .reduce(Rect::union)
}

/// The reserved band at the top of a group with a title: padding plus the title's height.
pub fn title_band(g: &Shape) -> Option<Rect> {
    (g.kind == ShapeKind::Group && g.title_band > 0.0)
        .then(|| Rect::new(g.rect.x, g.rect.y, g.rect.w, g.title_band))
}

/// Where a group's title is drawn: the union of its label, sublabel and stacked block items.
pub fn title_box(g: &Shape) -> Option<Rect> {
    if g.kind != ShapeKind::Group {
        return None;
    }
    g.text
        .iter()
        .filter(|t| t.role != TextRole::Absolute)
        .map(|t| t.rect)
        .reduce(Rect::union)
}
