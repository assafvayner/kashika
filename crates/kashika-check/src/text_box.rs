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
        .reduce(union)
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = (a.right().max(b.right()), a.bottom().max(b.bottom()));
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}
