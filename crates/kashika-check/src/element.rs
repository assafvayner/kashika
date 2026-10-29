//! The elements diagnostics name, with the geometry the JSON report carries.

use kashika_diag::Element;
use kashika_geom::{Pt, Rect, round2};
use kashika_model::{Junction, Line, Shape, ShapeKind, Text};

use crate::text_box::text_box;

pub fn arr(p: Pt) -> [f64; 2] {
    [round2(p.x), round2(p.y)]
}

pub fn bbox(r: Rect) -> [[f64; 2]; 2] {
    [arr(r.origin()), arr(Pt::new(r.right(), r.bottom()))]
}

pub fn kind_name(s: &Shape) -> &'static str {
    match s.kind {
        ShapeKind::Box => "box",
        ShapeKind::Group => "group",
        ShapeKind::Image => "image",
    }
}

/// The id of a text, or `text@L<n>` when it has none.
pub fn text_id(t: &Text) -> String {
    t.id.clone()
        .unwrap_or_else(|| format!("text@L{}", t.span.line))
}

pub fn shape_el(s: &Shape) -> Element {
    let mut e = Element::new(s.id.clone(), kind_name(s), s.span);
    e.bbox = Some(bbox(s.rect));
    if let Some(p) = &s.parent {
        e.frame = Some(p.clone());
        e.local_bbox = Some(bbox(s.local_rect()));
    }
    e
}

/// A line element; `seg` is an index into `l.segments`.
pub fn line_el(l: &Line, seg: Option<usize>) -> Element {
    let mut e = Element::new(l.id.clone(), "line", l.span);
    if let Some((i, s)) = seg.and_then(|i| l.segments.get(i).map(|s| (i, s))) {
        e.segment = Some(i + 1);
        e.from = Some(arr(s.from));
        e.to = Some(arr(s.to));
    }
    e
}

pub fn junction_el(j: &Junction) -> Element {
    let mut e = Element::new(j.id.clone(), "junction", j.span);
    e.bbox = Some(bbox(Rect::new(j.at.x, j.at.y, 0.0, 0.0)));
    e
}

pub fn text_el(t: &Text) -> Element {
    let mut e = Element::new(text_id(t), "text", t.span);
    e.bbox = Some(bbox(text_box(t)));
    e
}
