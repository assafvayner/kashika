use kashika_diag::Span;
use kashika_geom::{PathSeg, Pt, Rect, TOLERANCE, convex_hull};
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

impl Scene {
    /// Indices into `shapes` in paint order: top-level `layer=under` shapes, each group followed
    /// by its own `layer=under` children, the other boxes and images, then `layer=over` shapes.
    /// Junction dots, lines, line labels and texts paint before the `layer=over` shapes.
    pub fn paint_order(&self) -> Vec<usize> {
        let groups: Vec<&str> = self
            .shapes
            .iter()
            .filter(|g| g.kind == ShapeKind::Group)
            .map(|g| g.id.as_str())
            .collect();
        let under: Vec<(usize, Option<&str>)> = self
            .shapes
            .iter()
            .enumerate()
            .filter(|(_, s)| s.layer == Layer::Under)
            .map(|(i, s)| (i, s.parent.as_deref().filter(|p| groups.contains(p))))
            .collect();
        let under_in = |parent: Option<&str>| {
            under
                .iter()
                .filter(move |(_, p)| *p == parent)
                .map(|&(i, _)| i)
                .collect::<Vec<_>>()
        };
        let mut order = under_in(None);
        for (i, g) in self.shapes.iter().enumerate() {
            if g.kind == ShapeKind::Group {
                order.push(i);
                order.extend(under_in(Some(&g.id)));
            }
        }
        for layer in [Layer::Normal, Layer::Over] {
            order.extend(
                self.shapes
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.kind != ShapeKind::Group && s.layer == layer)
                    .map(|(i, _)| i),
            );
        }
        order
    }
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
    /// Before everything else in its parent: a top-level shape before every group, a group's
    /// child right after that group's own fill and border. Not an obstacle.
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
#[serde(rename_all = "kebab-case")]
pub enum Marker {
    None,
    Arrow,
    Open,
    Diamond,
    Dot,
    One,
    Many,
    OneMany,
    ZeroOne,
    ZeroMany,
}

/// The strokes of a crow's-foot end: bars and prongs as (from, to), and the open circle of
/// the `zero-*` ends as (center, radius).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ErParts {
    pub tip: Pt,
    pub strokes: Vec<(Pt, Pt)>,
    pub circle: Option<(Pt, f64)>,
}

impl Marker {
    pub const DOT_RADIUS: f64 = 3.0;
    pub const CIRCLE_RADIUS: f64 = 4.0;
    const ARROW_LEN: f64 = 10.0;
    const DIAMOND_LEN: f64 = 12.0;
    const HALF_WIDTH: f64 = 4.0;
    const FOOT_LEN: f64 = 12.0;
    const ER_HALF_WIDTH: f64 = 6.0;
    const BAR_AT: f64 = 8.0;
    const ER_GAP: f64 = 4.0;

    pub const ALL: [Marker; 10] = [
        Marker::None,
        Marker::Arrow,
        Marker::Open,
        Marker::Diamond,
        Marker::Dot,
        Marker::One,
        Marker::Many,
        Marker::OneMany,
        Marker::ZeroOne,
        Marker::ZeroMany,
    ];

    /// The `start=`/`end=` names, in `ALL` order.
    pub const NAMES: &'static [&'static str] = &{
        let mut names = [""; Self::ALL.len()];
        let mut i = 0;
        while i < names.len() {
            names[i] = Self::ALL[i].name();
            i += 1;
        }
        names
    };

    pub fn from_name(name: &str) -> Option<Marker> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Marker::None => "none",
            Marker::Arrow => "arrow",
            Marker::Open => "open",
            Marker::Diamond => "diamond",
            Marker::Dot => "dot",
            Marker::One => "one",
            Marker::Many => "many",
            Marker::OneMany => "one-many",
            Marker::ZeroOne => "zero-one",
            Marker::ZeroMany => "zero-many",
        }
    }

    /// How far the line stroke stops short of the tip. Crow's-foot ends sit on the line and
    /// don't trim it.
    pub fn length(self) -> f64 {
        match self {
            Marker::Arrow => Self::ARROW_LEN,
            Marker::Diamond => Self::DIAMOND_LEN,
            _ => 0.0,
        }
    }

    /// How far the marker reaches to either side of the line.
    pub fn half_width(self) -> f64 {
        match self {
            Marker::None => 0.0,
            Marker::Dot => Self::DOT_RADIUS,
            Marker::Arrow | Marker::Open | Marker::Diamond => Self::HALF_WIDTH,
            Marker::One | Marker::Many | Marker::OneMany | Marker::ZeroOne | Marker::ZeroMany => {
                Self::ER_HALF_WIDTH
            }
        }
    }

    /// The bars, prongs and circle of a crow's-foot end. `back(d)` is the point `d` back
    /// along the line from the tip and the line's direction there, toward the tip; the tip
    /// is `back(0)`. Bars cross the line where they sit, so they stay on a curve. Empty for
    /// the other markers.
    pub fn er_parts(self, back: impl Fn(f64) -> (Pt, Pt)) -> ErParts {
        let across = |d: f64| {
            let (p, dir) = back(d);
            let u = dir.norm();
            (p, Pt::new(-u.y, u.x) * Self::ER_HALF_WIDTH)
        };
        let bar = |d: f64| {
            let (p, perp) = across(d);
            (p + perp, p - perp)
        };
        let foot = || {
            let (tip, perp) = across(0.0);
            let apex = back(Self::FOOT_LEN).0;
            vec![(apex, tip + perp), (apex, tip), (apex, tip - perp)]
        };
        let circle = |d: f64| {
            Some((
                back(d + Self::ER_GAP + Self::CIRCLE_RADIUS).0,
                Self::CIRCLE_RADIUS,
            ))
        };
        let (strokes, circle) = match self {
            Marker::One => (vec![bar(Self::BAR_AT)], None),
            Marker::Many => (foot(), None),
            Marker::OneMany => {
                let mut s = foot();
                s.push(bar(Self::FOOT_LEN + Self::ER_GAP));
                (s, None)
            }
            Marker::ZeroOne => (vec![bar(Self::BAR_AT)], circle(Self::BAR_AT)),
            Marker::ZeroMany => (foot(), circle(Self::FOOT_LEN)),
            Marker::None | Marker::Arrow | Marker::Open | Marker::Diamond | Marker::Dot => {
                (Vec::new(), None)
            }
        };
        ErParts {
            tip: back(0.0).0,
            strokes,
            circle,
        }
    }

    /// Vertices of the head with its tip on `tip` pointing along `dir`: the tip first, then
    /// the rest in drawing order. For crow's-foot ends on a straight end segment, the
    /// convex hull of their parts. Empty for `None` and `Dot`.
    pub fn outline(self, tip: Pt, dir: Pt) -> Vec<Pt> {
        let u = dir.norm();
        let perp = Pt::new(-u.y, u.x) * Self::HALF_WIDTH;
        match self {
            Marker::Arrow | Marker::Open => {
                let base = tip - u * Self::ARROW_LEN;
                vec![tip, base + perp, base - perp]
            }
            Marker::Diamond => {
                let mid = tip - u * (Self::DIAMOND_LEN / 2.0);
                vec![tip, mid + perp, tip - u * Self::DIAMOND_LEN, mid - perp]
            }
            Marker::None | Marker::Dot => Vec::new(),
            Marker::One | Marker::Many | Marker::OneMany | Marker::ZeroOne | Marker::ZeroMany => {
                self.er_parts(|d| (tip - u * d, u)).hull()
            }
        }
    }
}

impl ErParts {
    /// The convex hull of the tip, the bars, the prongs and the circle's bounding square, or
    /// nothing when there are no parts.
    pub fn hull(&self) -> Vec<Pt> {
        if self.strokes.is_empty() {
            return Vec::new();
        }
        let mut pts: Vec<Pt> = self.strokes.iter().flat_map(|&(a, b)| [a, b]).collect();
        pts.push(self.tip);
        if let Some((c, r)) = self.circle {
            pts.extend([
                c + Pt::new(r, r),
                c + Pt::new(r, -r),
                c + Pt::new(-r, r),
                c + Pt::new(-r, -r),
            ]);
        }
        convex_hull(&pts)
    }
}

/// Where a line's labels sit: on the line, masking it, or beside it (`label-side=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelSide {
    #[default]
    On,
    Above,
    Below,
    Left,
    Right,
}

/// The direction of the path piece under a label's point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
    Diagonal,
    Curve,
}

impl Orientation {
    pub fn name(self) -> &'static str {
        match self {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
            Orientation::Diagonal => "diagonal",
            Orientation::Curve => "curved",
        }
    }

    fn of(seg: &PathSeg) -> Orientation {
        match *seg {
            PathSeg::Line { from, to } if (from.y - to.y).abs() <= TOLERANCE => {
                Orientation::Horizontal
            }
            PathSeg::Line { from, to } if (from.x - to.x).abs() <= TOLERANCE => {
                Orientation::Vertical
            }
            PathSeg::Line { .. } => Orientation::Diagonal,
            _ => Orientation::Curve,
        }
    }
}

/// The orientations of the path pieces at fraction `t` of the arc length: one, or two
/// at a vertex.
pub fn orientations_at(path: &[PathSeg], t: f64) -> Vec<Orientation> {
    let lens: Vec<f64> = path.iter().map(PathSeg::length).collect();
    let target = lens.iter().sum::<f64>() * t.clamp(0.0, 1.0);
    let mut start = 0.0;
    let mut out = Vec::new();
    for (seg, len) in path.iter().zip(lens) {
        if target >= start - TOLERANCE && target <= start + len + TOLERANCE {
            out.push(Orientation::of(seg));
        }
        start += len;
    }
    out
}

impl LabelSide {
    pub const NAMES: &'static [&'static str] = &["on", "above", "below", "left", "right"];

    pub fn from_name(name: &str) -> Option<LabelSide> {
        [
            LabelSide::On,
            LabelSide::Above,
            LabelSide::Below,
            LabelSide::Left,
            LabelSide::Right,
        ]
        .into_iter()
        .find(|s| s.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            LabelSide::On => "on",
            LabelSide::Above => "above",
            LabelSide::Below => "below",
            LabelSide::Left => "left",
            LabelSide::Right => "right",
        }
    }

    pub fn is_on(&self) -> bool {
        *self == LabelSide::On
    }

    /// The segment orientation this side needs; `None` for `on`, which fits anywhere.
    pub fn needs(self) -> Option<Orientation> {
        match self {
            LabelSide::On => None,
            LabelSide::Above | LabelSide::Below => Some(Orientation::Horizontal),
            LabelSide::Left | LabelSide::Right => Some(Orientation::Vertical),
        }
    }

    /// Checks that a label at fraction `t` of `path` can sit on this side. On a mismatch,
    /// returns the orientation of the piece under the point (an axis-aligned one at a vertex).
    pub fn fits(self, path: &[PathSeg], t: f64) -> Result<(), Orientation> {
        let Some(need) = self.needs() else {
            return Ok(());
        };
        let found = orientations_at(path, t);
        if found.contains(&need) {
            return Ok(());
        }
        Err(found
            .iter()
            .copied()
            .find(|o| matches!(o, Orientation::Horizontal | Orientation::Vertical))
            .or(found.first().copied())
            .unwrap_or(Orientation::Curve))
    }

    /// From the point on the line to the center of a `w`×`h` label on this side: its near
    /// edge sits `stroke_width / 2 + 4` from the line.
    pub fn offset(self, w: f64, h: f64, stroke_width: f64) -> Pt {
        let gap = stroke_width / 2.0 + LABEL_GAP;
        match self {
            LabelSide::On => Pt::new(0.0, 0.0),
            LabelSide::Above => Pt::new(0.0, -(gap + h / 2.0)),
            LabelSide::Below => Pt::new(0.0, gap + h / 2.0),
            LabelSide::Left => Pt::new(-(gap + w / 2.0), 0.0),
            LabelSide::Right => Pt::new(gap + w / 2.0, 0.0),
        }
    }
}

/// The space between a line's stroke and a label placed beside it.
pub const LABEL_GAP: f64 = 4.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Label {
    pub t: f64,
    pub text: String,
    /// The point on the path at `t`; the label's center when it sits on the line.
    pub at: Pt,
    pub rect: Rect,
    pub lines: Vec<PlacedLine>,
    /// Where this label sits: the line's `label-side`, or `on` when that side does not
    /// fit the segment under `at`.
    #[serde(skip_serializing_if = "LabelSide::is_on")]
    pub side: LabelSide,
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
    #[serde(skip_serializing_if = "LabelSide::is_on")]
    pub label_side: LabelSide,
    pub ortho: bool,
    pub allow: Vec<Allow>,
    pub reason: Option<String>,
    pub parent: Option<String>,
    pub origin: Pt,
    pub span: Span,
}

impl Line {
    /// The crow's-foot parts of the start (`start`) or end marker, laid along the end
    /// segment. Past that segment they continue straight along its far end.
    pub fn er_parts(&self, start: bool) -> ErParts {
        let (m, seg) = if start {
            (self.start, self.path.first())
        } else {
            (self.end, self.path.last())
        };
        let Some(&seg) = seg else {
            return ErParts::default();
        };
        let len = seg.length();
        m.er_parts(|d| {
            if start {
                if d < len {
                    let s = seg.trim(d, 0.0);
                    (s.from(), s.start_dir() * -1.0)
                } else {
                    let u = seg.end_dir();
                    (seg.to() + u * (d - len), u * -1.0)
                }
            } else if d < len {
                let s = seg.trim(0.0, d);
                (s.to(), s.end_dir())
            } else {
                let u = seg.start_dir();
                (seg.from() - u * (d - len), u)
            }
        })
    }
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
    /// The color an SVG icon's `currentColor` resolved to: the box's text color.
    #[serde(skip)]
    pub current_color: Option<String>,
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
