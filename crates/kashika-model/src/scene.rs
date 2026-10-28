use kashika_diag::Span;
use kashika_geom::{PathSeg, Pt, Rect};
use kashika_text::PlacedLine;
use serde::Serialize;

use crate::style::Style;
use crate::textlayout::TextItem;

#[derive(Debug, Clone, Serialize)]
pub struct Scene {
    pub title: String,
    pub size: (f64, f64),
    pub grid: Option<f64>,
    pub clearance: f64,
    pub allow: Vec<Allow>,
    pub shapes: Vec<Shape>,
    pub junctions: Vec<Junction>,
    pub texts: Vec<Text>,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Box,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outline {
    Rect,
    Rounded,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shape {
    pub id: String,
    pub kind: ShapeKind,
    pub outline: Outline,
    pub label: Option<String>,
    pub rect: Rect,
    pub parent: Option<String>,
    pub depth: usize,
    pub style: Style,
    /// The bounding box inset by `padding` (and by the shape's own reduction).
    pub text_area: Rect,
    /// Label, sublabel and text block items, laid out.
    pub text: Vec<TextItem>,
    /// Height of a group's reserved title band from its top edge; 0 for boxes and
    /// unlabeled groups.
    pub title_band: f64,
    pub span: Span,
}

impl Shape {
    pub fn corner_radius(&self) -> f64 {
        corner_radius(self.style.radius, self.rect)
    }
}

pub(crate) fn corner_radius(radius: f64, rect: Rect) -> f64 {
    radius.min(rect.w / 2.0).min(rect.h / 2.0).max(0.0)
}

#[derive(Debug, Clone, Serialize)]
pub struct Junction {
    pub id: String,
    pub at: Pt,
    pub dot: bool,
    pub parent: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub id: Option<String>,
    pub content: String,
    pub at: Pt,
    pub size: f64,
    pub color: String,
    pub align: Align,
    pub rect: Rect,
    pub lines: Vec<PlacedLine>,
    pub parent: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Marker {
    None,
    Arrow,
    Open,
    Diamond,
    Dot,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Label {
    pub t: f64,
    pub text: String,
    /// The point on the path at `t`; the label's center.
    pub at: Pt,
    pub rect: Rect,
    pub lines: Vec<PlacedLine>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Allow {
    pub code: String,
    pub other: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    /// The declared id, or `line@L<n>` for unnamed lines.
    pub id: String,
    pub named: bool,
    pub path: Vec<PathSeg>,
    pub start: Marker,
    pub end: Marker,
    pub style: Style,
    pub labels: Vec<Label>,
    pub ortho: bool,
    pub allow: Vec<Allow>,
    pub reason: Option<String>,
    pub parent: Option<String>,
    pub span: Span,
}

impl Shape {
    /// The canvas position of a plain anchor (`top`, `tl`, ...) on this shape, resolved the
    /// same way as anchors written in a file.
    pub fn anchor(&self, name: &str) -> Option<Pt> {
        crate::anchor::resolve(self.rect, self.corner_radius(), &self.id, name, None).ok()
    }
}
