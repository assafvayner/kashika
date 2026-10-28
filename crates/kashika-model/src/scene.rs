use kashika_diag::Span;
use kashika_geom::{PathSeg, Pt, Rect};
use kashika_text::PlacedLine;
use serde::Serialize;

use crate::form::Form;
use crate::style::Style;
use crate::textlayout::TextItem;

#[derive(Debug, Clone, Serialize)]
pub struct Scene {
    pub title: String,
    pub size: (f64, f64),
    pub grid: Option<f64>,
    pub clearance: f64,
    /// The canvas color; `None` leaves the canvas transparent.
    pub background: Option<String>,
    pub allow: Vec<Allow>,
    pub shapes: Vec<Shape>,
    pub junctions: Vec<Junction>,
    pub texts: Vec<Text>,
    pub lines: Vec<Line>,
    /// Embedded files (images, icons, SVG shapes), in first-use order.
    pub assets: Vec<Asset>,
    /// Who is placed relative to whom: `at` references and group frames.
    pub placement: Vec<PlacementEdge>,
    /// Every number the author wrote as a position, distance or size.
    pub literals: Vec<Literal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Box,
    Group,
    Image,
}

/// `layer=`: when a box or image paints, and whether checks treat it as an obstacle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// Before everything else, groups included; not an obstacle.
    Under,
    #[default]
    Normal,
    /// After everything else, texts and line labels included; not an obstacle.
    Over,
}

impl Layer {
    pub fn is_normal(&self) -> bool {
        *self == Layer::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outline {
    Rect,
    Rounded,
    /// A built-in, custom or SVG shape; `Shape::form` holds the geometry.
    Path,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shape {
    pub id: String,
    pub kind: ShapeKind,
    pub outline: Outline,
    /// `rect`, `rounded`, a built-in shape, a defined or imported shape (`aws.s3`), or `image`.
    pub shape: String,
    /// Outline and drawing for `Outline::Path` shapes.
    pub form: Option<Form>,
    /// The drawing of an `image` or a `shape from` SVG.
    pub image: Option<ImageRef>,
    pub icon: Option<ImageRef>,
    pub label: Option<String>,
    pub rect: Rect,
    pub parent: Option<String>,
    pub depth: usize,
    #[serde(skip_serializing_if = "Layer::is_normal")]
    pub layer: Layer,
    pub style: Style,
    /// The bounding box inset by `padding` (and by the shape's own reduction).
    pub text_area: Rect,
    /// `text-area=below`: the text sits in a band under the box instead of inside it.
    pub text_below: bool,
    /// Label, sublabel and text block items, laid out.
    pub text: Vec<TextItem>,
    /// Height of a group's reserved title band from its top edge; 0 for boxes and
    /// unlabeled groups.
    pub title_band: f64,
    pub allow: Vec<Allow>,
    pub reason: Option<String>,
    /// Canvas position of the frame this element's literals are written in.
    pub origin: Pt,
    pub span: Span,
}

impl Shape {
    pub fn corner_radius(&self) -> f64 {
        corner_radius(self.style.radius, self.rect)
    }

    /// The bounding box in the frame the shape was written in.
    pub fn local_rect(&self) -> Rect {
        Rect::new(
            self.rect.x - self.origin.x,
            self.rect.y - self.origin.y,
            self.rect.w,
            self.rect.h,
        )
    }

    /// Canvas position of an anchor, or `None` when the name or `t` is invalid.
    pub fn anchor(&self, name: &str, t: Option<f64>) -> Option<Pt> {
        crate::form::resolve_anchor(
            self.form.as_ref(),
            self.rect,
            self.style.radius,
            &self.id,
            name,
            t,
        )
        .ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    /// `from`'s `at` references `to`.
    At,
    /// `from` is written inside group `to`.
    Frame,
}

impl EdgeKind {
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::At => "at",
            EdgeKind::Frame => "frame",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlacementEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

/// A number the author wrote as a position, distance or size, in its own frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Literal {
    /// Id of the element the number belongs to (`line@L<n>`/`text@L<n>` when unnamed).
    pub owner: String,
    /// `x`, `y`, `dx`, `dy`, `h`, `v`, `width` or `height`.
    pub what: &'static str,
    pub value: f64,
    /// The enclosing point, segment or statement.
    pub span: Span,
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
    pub origin: Pt,
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
    pub origin: Pt,
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
    /// The authored segments, in order.
    pub segments: Vec<Segment>,
    /// For each piece of `path`, the index into `segments` that drew it.
    pub path_seg: Vec<usize>,
    /// Whether the start point was written as a literal or mixed point rather than an anchor or junction.
    pub start_literal: bool,
    /// Whether the last segment ended on a literal: `h`/`v`, a numeric `x`/`y`, or a literal target point.
    pub end_literal: bool,
    pub start: Marker,
    pub end: Marker,
    pub style: Style,
    pub labels: Vec<Label>,
    pub ortho: bool,
    pub allow: Vec<Allow>,
    pub reason: Option<String>,
    pub parent: Option<String>,
    pub origin: Pt,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    Contain,
    Cover,
    Stretch,
}

/// A placed asset: which file, where, and how it fills its rect.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageRef {
    /// Index into `Scene::assets`.
    pub asset: usize,
    pub rect: Rect,
    pub fit: Fit,
}

#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    /// The path as written in the source, or `builtin:<name>` for a bundled icon.
    pub path: String,
    pub mime: &'static str,
    /// Natural size in px (SVG: its width/height; raster: pixels).
    pub natural: (f64, f64),
    /// The bytes embedded in output: sanitized SVG, or the raster file unchanged.
    #[serde(skip)]
    pub bytes: Vec<u8>,
    /// Convex hull of the drawing in unit coordinates (the unit square for rasters).
    #[serde(skip)]
    pub unit_hull: Vec<Pt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    To,
    H,
    V,
    X,
    Y,
    Hv,
    Vh,
    Curve,
    CurveVia,
}

impl SegmentKind {
    pub fn name(self) -> &'static str {
        match self {
            SegmentKind::To => "to",
            SegmentKind::H => "h",
            SegmentKind::V => "v",
            SegmentKind::X => "x",
            SegmentKind::Y => "y",
            SegmentKind::Hv => "hv",
            SegmentKind::Vh => "vh",
            SegmentKind::Curve => "curve",
            SegmentKind::CurveVia => "curve-via",
        }
    }
}

/// One authored `-> ...` segment of a line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Segment {
    pub kind: SegmentKind,
    pub span: Span,
    pub from: Pt,
    pub to: Pt,
    /// The geometry this segment drew before corner rounding, without zero-length pieces.
    pub pieces: Vec<PathSeg>,
}
