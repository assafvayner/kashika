use std::collections::HashMap;

use kashika_diag::Element;
use kashika_geom::{PathSeg, Pt, Rect, TOLERANCE, boundary_nearest, pieces, point_in_polygon};
use kashika_model::{Junction, Line, Scene, Shape, ShapeKind, Text};

use crate::index::Grid;
use crate::outline::polygon;
use crate::text_box::{below_box, label_box, text_box};

pub struct ShapeGeom<'a> {
    pub shape: &'a Shape,
    pub poly: Vec<Pt>,
}

pub struct LineGeom<'a> {
    pub line: &'a Line,
    /// Straight pieces of the rendered path, curves flattened: (from, to, index into `line.path`).
    pub pieces: Vec<(Pt, Pt, usize)>,
    pub start: Pt,
    pub end: Pt,
}

impl LineGeom<'_> {
    /// Index into `line.segments` of the authored segment that drew `line.path[path_index]`.
    pub fn segment(&self, path_index: usize) -> usize {
        self.line.path_seg.get(path_index).copied().unwrap_or(0)
    }

    pub fn last_segment(&self) -> usize {
        self.line.segments.len().saturating_sub(1)
    }

    pub fn straight(&self, path_index: usize) -> bool {
        matches!(self.line.path.get(path_index), Some(PathSeg::Line { .. }))
    }

    pub fn straight_ends(&self, path_index: usize) -> Option<(Pt, Pt)> {
        match self.line.path.get(path_index) {
            Some(&PathSeg::Line { from, to }) => Some((from, to)),
            _ => None,
        }
    }
}

/// The scene plus geometry shared by every lint.
pub struct Ctx<'a> {
    pub scene: &'a Scene,
    pub shapes: Vec<ShapeGeom<'a>>,
    pub lines: Vec<LineGeom<'a>>,
    /// Bounding boxes of each shape (outline and rect) and line (pieces and ends), and grids over them.
    pub shape_boxes: Vec<Rect>,
    pub line_boxes: Vec<Rect>,
    shape_grid: Grid,
    line_grid: Grid,
    /// Index into `scene.shapes` of the first shape with each id.
    shape_ids: HashMap<&'a str, usize>,
    /// Index into `lines` of the first line with each id.
    line_ids: HashMap<&'a str, usize>,
    /// Index into `scene.lines`, `scene.junctions` and `scene.texts` of the first of each id.
    scene_line_ids: HashMap<&'a str, usize>,
    junction_ids: HashMap<&'a str, usize>,
    text_ids: HashMap<String, usize>,
    junction_grid: Grid,
    marks: Vec<Mark<'a>>,
    mark_grid: Grid,
}

/// Something drawn on top that a free lane must keep clear of besides boxes and lines: a
/// junction, a text, a line label or the label under a `text-area=below` shape.
pub struct Mark<'a> {
    pub rect: Rect,
    /// The id of the junction, text, line or shape it belongs to.
    pub owner: &'a str,
}

impl<'a> Ctx<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        let shapes: Vec<ShapeGeom> = scene
            .shapes
            .iter()
            .map(|s| ShapeGeom {
                shape: s,
                poly: polygon(s),
            })
            .collect();
        let lines: Vec<LineGeom> = scene
            .lines
            .iter()
            .filter(|l| !l.path.is_empty())
            .map(|l| LineGeom {
                line: l,
                pieces: pieces(&l.path),
                start: l.path[0].from(),
                end: l.path[l.path.len() - 1].to(),
            })
            .collect();
        let shape_boxes: Vec<Rect> = shapes
            .iter()
            .map(|s| {
                let r = s.shape.rect;
                let corners = [r.origin(), Pt::new(r.right(), r.bottom())];
                extent(s.poly.iter().copied().chain(corners))
            })
            .collect();
        let line_boxes: Vec<Rect> = lines
            .iter()
            .map(|l| {
                let ends = [l.start, l.end];
                extent(l.pieces.iter().flat_map(|&(p, q, _)| [p, q]).chain(ends))
            })
            .collect();
        let mut shape_ids = HashMap::new();
        for (i, s) in scene.shapes.iter().enumerate() {
            shape_ids.entry(s.id.as_str()).or_insert(i);
        }
        let mut line_ids = HashMap::new();
        for (i, l) in lines.iter().enumerate() {
            line_ids.entry(l.line.id.as_str()).or_insert(i);
        }
        let mut scene_line_ids = HashMap::new();
        for (i, l) in scene.lines.iter().enumerate() {
            scene_line_ids.entry(l.id.as_str()).or_insert(i);
        }
        let mut junction_ids = HashMap::new();
        for (i, j) in scene.junctions.iter().enumerate() {
            junction_ids.entry(j.id.as_str()).or_insert(i);
        }
        let mut text_ids = HashMap::new();
        for (i, t) in scene.texts.iter().enumerate() {
            text_ids.entry(text_id(t)).or_insert(i);
        }
        let junction_boxes: Vec<Rect> = scene
            .junctions
            .iter()
            .map(|j| Rect::new(j.at.x, j.at.y, 0.0, 0.0))
            .collect();
        let marks = marks(scene);
        let mark_boxes: Vec<Rect> = marks.iter().map(|m| m.rect).collect();
        Ctx {
            scene,
            shape_grid: Grid::new(&shape_boxes),
            line_grid: Grid::new(&line_boxes),
            scene_line_ids,
            junction_ids,
            text_ids,
            junction_grid: Grid::new(&junction_boxes),
            mark_grid: Grid::new(&mark_boxes),
            marks,
            shapes,
            lines,
            shape_boxes,
            line_boxes,
            shape_ids,
            line_ids,
        }
    }

    /// Indices into `shapes`, ascending, of the shapes whose outline may lie within `margin` of `r`.
    pub fn shapes_near(&self, r: Rect, margin: f64) -> Vec<usize> {
        self.shape_grid.near(r, margin + 1.0)
    }

    /// Indices into `lines`, ascending, of the lines whose pieces may lie within `margin` of `r`.
    pub fn lines_near(&self, r: Rect, margin: f64) -> Vec<usize> {
        self.line_grid.near(r, margin + 1.0)
    }

    /// Boxes and images whose outline may lie within `margin` of `r`, in `boxes()` order.
    pub fn boxes_near(&self, r: Rect, margin: f64) -> impl Iterator<Item = &ShapeGeom<'a>> + '_ {
        self.shapes_near(r, margin)
            .into_iter()
            .map(|i| &self.shapes[i])
            .filter(|s| s.shape.kind != ShapeKind::Group)
    }

    /// Groups whose outline may lie within `margin` of `r`, in `groups()` order.
    pub fn groups_near(&self, r: Rect, margin: f64) -> impl Iterator<Item = &ShapeGeom<'a>> + '_ {
        self.shapes_near(r, margin)
            .into_iter()
            .map(|i| &self.shapes[i])
            .filter(|s| s.shape.kind == ShapeKind::Group)
    }

    /// Boxes and images: the opaque shapes that lines and labels must stay clear of.
    pub fn boxes(&self) -> impl Iterator<Item = &ShapeGeom<'a>> + '_ {
        self.shapes
            .iter()
            .filter(|s| s.shape.kind != ShapeKind::Group)
    }

    pub fn groups(&self) -> impl Iterator<Item = &ShapeGeom<'a>> + '_ {
        self.shapes
            .iter()
            .filter(|s| s.shape.kind == ShapeKind::Group)
    }

    /// Boxes that `p` lies on or inside: the boxes a line ending at `p` attaches to.
    pub fn attached(&self, p: Pt) -> Vec<&'a str> {
        self.boxes_near(Rect::new(p.x, p.y, 0.0, 0.0), TOLERANCE)
            .filter(|s| point_in_polygon(p, &s.poly) || boundary_nearest(p, &s.poly).0 <= TOLERANCE)
            .map(|s| s.shape.id.as_str())
            .collect()
    }

    /// Junctions within TOLERANCE of `p`, in file order.
    pub fn junctions_at(&self, p: Pt) -> impl Iterator<Item = &'a Junction> + '_ {
        self.junctions_near(Rect::new(p.x, p.y, 0.0, 0.0), TOLERANCE)
            .filter(move |j| j.at.dist(p) <= TOLERANCE)
    }

    /// Junctions that may lie within `margin` of `r`, in file order.
    pub fn junctions_near(&self, r: Rect, margin: f64) -> impl Iterator<Item = &'a Junction> + '_ {
        self.junction_grid
            .near(r, margin + 1.0)
            .into_iter()
            .map(|i| &self.scene.junctions[i])
    }

    pub fn is_junction(&self, p: Pt) -> bool {
        self.junctions_at(p).next().is_some()
    }

    /// Junctions, texts and labels whose box may lie within `margin` of `r`, in `marks()` order.
    pub fn marks_near(&self, r: Rect, margin: f64) -> impl Iterator<Item = &Mark<'a>> + '_ {
        self.mark_grid
            .near(r, margin + 1.0)
            .into_iter()
            .map(|i| &self.marks[i])
    }

    pub fn shape(&self, id: &str) -> Option<&'a Shape> {
        self.shape_index(id).map(|i| &self.scene.shapes[i])
    }

    /// Index into `shapes` of the first shape with id `id`.
    pub fn shape_index(&self, id: &str) -> Option<usize> {
        self.shape_ids.get(id).copied()
    }

    pub fn line(&self, id: &str) -> Option<&LineGeom<'a>> {
        self.line_ids.get(id).map(|&i| &self.lines[i])
    }

    /// True when `anc` is a parent, grandparent, ... of shape `id`.
    pub fn is_ancestor(&self, anc: &str, id: &str) -> bool {
        let mut cur = self.shape(id).and_then(|s| s.parent.as_deref());
        while let Some(p) = cur {
            if p == anc {
                return true;
            }
            cur = self.shape(p).and_then(|s| s.parent.as_deref());
        }
        false
    }

    /// An element for any id in the scene.
    pub fn element(&self, id: &str) -> Option<Element> {
        if let Some(s) = self.shape(id) {
            return Some(shape_el(s));
        }
        if let Some(&i) = self.scene_line_ids.get(id) {
            return Some(line_el(&self.scene.lines[i], None));
        }
        if let Some(&i) = self.junction_ids.get(id) {
            return Some(junction_el(&self.scene.junctions[i]));
        }
        self.text_ids
            .get(id)
            .map(|&i| text_el(&self.scene.texts[i]))
    }
}

/// Junctions, then texts, line labels and the labels under `text-area=below` shapes.
fn marks(scene: &Scene) -> Vec<Mark<'_>> {
    let junctions = scene.junctions.iter().map(|j| Mark {
        rect: Rect::new(j.at.x, j.at.y, 0.0, 0.0),
        owner: j.id.as_str(),
    });
    let texts = scene.texts.iter().map(|t| Mark {
        rect: text_box(t),
        owner: t.id.as_deref().unwrap_or(""),
    });
    let labels = scene.lines.iter().flat_map(|l| {
        l.labels.iter().map(|lab| Mark {
            rect: label_box(lab),
            owner: l.id.as_str(),
        })
    });
    let below = scene.shapes.iter().filter_map(|s| {
        below_box(s).map(|rect| Mark {
            rect,
            owner: s.id.as_str(),
        })
    });
    junctions.chain(texts).chain(labels).chain(below).collect()
}

pub fn round2(v: f64) -> f64 {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 { 0.0 } else { r }
}

pub fn fmt_num(v: f64) -> String {
    format!("{}", round2(v))
}

pub fn fmt_pt(p: Pt) -> String {
    format!("({},{})", fmt_num(p.x), fmt_num(p.y))
}

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

/// Whether `inner` lies within `outer`, touching allowed.
pub fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x - TOLERANCE
        && inner.y >= outer.y - TOLERANCE
        && inner.right() <= outer.right() + TOLERANCE
        && inner.bottom() <= outer.bottom() + TOLERANCE
}

/// The overlap of two rects, when it is more than TOLERANCE in both directions.
pub fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let (x0, y0) = (a.x.max(b.x), a.y.max(b.y));
    let (x1, y1) = (a.right().min(b.right()), a.bottom().min(b.bottom()));
    (x1 - x0 > TOLERANCE && y1 - y0 > TOLERANCE).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

pub fn points_bbox(pts: impl IntoIterator<Item = Pt>) -> Rect {
    let mut it = pts.into_iter();
    let Some(first) = it.next() else {
        return Rect::default();
    };
    let (mut lo, mut hi) = (first, first);
    for p in it {
        lo = Pt::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Pt::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    Rect::new(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y)
}

/// The bounding box of `pts`, or a NaN box when any of them is not finite.
fn extent(pts: impl Iterator<Item = Pt> + Clone) -> Rect {
    if pts.clone().all(|p| p.x.is_finite() && p.y.is_finite()) {
        points_bbox(pts)
    } else {
        Rect::new(f64::NAN, f64::NAN, 0.0, 0.0)
    }
}

pub fn line_bbox(l: &LineGeom) -> Rect {
    points_bbox(l.pieces.iter().flat_map(|&(p, q, _)| [p, q]))
}

/// How far `inner` sticks out of `outer` on each side, ignoring anything within TOLERANCE.
pub fn overflow(outer: Rect, inner: Rect) -> Vec<(&'static str, f64)> {
    [
        ("left", outer.x - inner.x),
        ("top", outer.y - inner.y),
        ("right", inner.right() - outer.right()),
        ("bottom", inner.bottom() - outer.bottom()),
    ]
    .into_iter()
    .filter(|(_, d)| *d > TOLERANCE)
    .map(|(s, d)| (s, round2(d)))
    .collect()
}

/// e.g. "right by 20px, bottom by 5px"
pub fn describe_overflow(over: &[(&str, f64)]) -> String {
    over.iter()
        .map(|(s, d)| format!("{s} by {}px", fmt_num(*d)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn overflow_fact(over: &[(&str, f64)]) -> serde_json::Value {
    serde_json::Value::Object(
        over.iter()
            .map(|(s, d)| (s.to_string(), serde_json::json!(d)))
            .collect(),
    )
}

/// The bounding-box side nearest to `p`, and `p`'s position along it (0..1).
pub fn side_anchor(s: &Shape, p: Pt) -> (&'static str, f64) {
    let r = s.rect;
    let sides = [
        ("top", (p.y - r.y).abs()),
        ("bottom", (p.y - r.bottom()).abs()),
        ("left", (p.x - r.x).abs()),
        ("right", (p.x - r.right()).abs()),
    ];
    let side = sides
        .iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or("top", |s| s.0);
    let t = match side {
        "top" | "bottom" => (p.x - r.x) / r.w,
        _ => (p.y - r.y) / r.h,
    };
    (
        side,
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.5
        },
    )
}

/// The bounding-box side nearest to `p`, and `p`'s position along it (0..1). On shapes
/// with a form, edge anchors project onto the outline, so the nearest side whose anchor
/// lands back on `p` wins.
pub fn outline_anchor(s: &Shape, p: Pt) -> (&'static str, f64) {
    let nearest = side_anchor(s, p);
    if s.form.is_none() {
        return nearest;
    }
    let r = s.rect;
    let mut sides = [
        ("top", (p.y - r.y).abs()),
        ("bottom", (p.y - r.bottom()).abs()),
        ("left", (p.x - r.x).abs()),
        ("right", (p.x - r.right()).abs()),
    ];
    sides.sort_by(|a, b| a.1.total_cmp(&b.1));
    sides
        .iter()
        .find_map(|&(side, _)| {
            let t = match side {
                "top" | "bottom" => (p.x - r.x) / r.w,
                _ => (p.y - r.y) / r.h,
            };
            let t = if t.is_finite() {
                t.clamp(0.0, 1.0)
            } else {
                0.5
            };
            s.anchor(side, Some(t))
                .filter(|q| q.dist(p) <= 0.5)
                .map(|_| (side, t))
        })
        .unwrap_or(nearest)
}

/// `id.side` or `id.side@t` for the anchor of `s` at outline point `p`.
pub fn anchor_text(s: &Shape, p: Pt) -> String {
    let (side, t) = outline_anchor(s, p);
    let t = round2(t);
    if t == 0.5 {
        format!("{}.{side}", s.id)
    } else {
        format!("{}.{side}@{t}", s.id)
    }
}
