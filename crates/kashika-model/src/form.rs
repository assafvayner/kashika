use kashika_geom::{PathSeg, Pt, Rect, arc_to_cubics, flatten, horizontal_hits, vertical_hits};
use serde::Serialize;

use crate::anchor;
use crate::scene::{Asset, Fit, Outline, Shape, ShapeKind, corner_radius};
use crate::shapes::UnitShape;

/// The geometry of a non-rectangular shape, in canvas coordinates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Form {
    /// Subpaths to fill and stroke; empty for SVG-file shapes, which draw their image instead.
    pub draw: Vec<Vec<PathSeg>>,
    /// Closed outline polygon, flattened, used for anchors and checks.
    pub outline: Vec<Pt>,
}

impl Shape {
    /// The shape's outline as a closed polygon in canvas coordinates, for anchors and checks.
    /// Rounded corners and curves are flattened to `FLATTEN_TOLERANCE`.
    pub fn outline_polygon(&self) -> Vec<Pt> {
        if let Some(f) = &self.form {
            return f.outline.clone();
        }
        match self.outline {
            Outline::Rounded => rounded_polygon(self.rect, self.corner_radius()),
            Outline::Rect | Outline::Path => corners(self.rect),
        }
    }

    /// How much `fit=stretch` changes an image's aspect ratio: |box ratio / natural ratio - 1|.
    /// `None` for other fits, non-images, and images whose file failed to load.
    pub fn image_distortion(&self, assets: &[Asset]) -> Option<f64> {
        if self.kind != ShapeKind::Image {
            return None;
        }
        let img = self.image.as_ref().filter(|i| i.fit == Fit::Stretch)?;
        let (nw, nh) = assets.get(img.asset)?.natural;
        if nw <= 0.0 || nh <= 0.0 || img.rect.h <= 0.0 {
            return None;
        }
        Some(((img.rect.w / img.rect.h) / (nw / nh) - 1.0).abs())
    }
}

impl Form {
    pub(crate) fn from_unit(u: &UnitShape, r: Rect) -> Form {
        let mut draw: Vec<Vec<PathSeg>> = u
            .subpaths
            .iter()
            .map(|sp| sp.iter().map(|s| s.map(|p| r.map_unit(p))).collect())
            .collect();
        let outline = closed(flatten(&draw[u.outline]));
        if u.hidden_outline {
            draw.remove(u.outline);
        }
        Form { draw, outline }
    }

    /// An SVG-file shape: nothing to draw (the image draws instead) and an outline that is
    /// the box, or the drawing's convex hull stretched onto the box.
    pub(crate) fn from_svg(asset: &Asset, hull: bool, r: Rect) -> Form {
        let outline = if hull {
            asset.unit_hull.iter().map(|p| r.map_unit(*p)).collect()
        } else {
            corners(r)
        };
        Form {
            draw: Vec::new(),
            outline,
        }
    }
}

/// Resolves an anchor on a placed box or group (§4). On shapes with a `form`, edge
/// anchors are moved onto the outline along their axis.
pub(crate) fn resolve_anchor(
    form: Option<&Form>,
    rect: Rect,
    radius: f64,
    owner: &str,
    name: &str,
    t: Option<f64>,
) -> Result<Pt, (String, Option<String>)> {
    match form {
        Some(f) => anchor::resolve(rect, 0.0, owner, name, t).map(|p| project(&f.outline, name, p)),
        None => anchor::resolve(rect, corner_radius(radius, rect), owner, name, t),
    }
}

/// Moves a bbox anchor point onto the outline along its axis: `top`/`bottom` move
/// vertically, `left`/`right` horizontally. Other anchors, and lines that miss the
/// outline, keep the bbox point.
pub(crate) fn project(poly: &[Pt], anchor: &str, p: Pt) -> Pt {
    let pick = |v: Vec<f64>, max: bool| {
        let f: fn(f64, f64) -> f64 = if max { f64::max } else { f64::min };
        v.into_iter().reduce(f)
    };
    match anchor {
        "top" => pick(vertical_hits(poly, p.x), false).map_or(p, |y| Pt::new(p.x, y)),
        "bottom" => pick(vertical_hits(poly, p.x), true).map_or(p, |y| Pt::new(p.x, y)),
        "left" => pick(horizontal_hits(poly, p.y), false).map_or(p, |x| Pt::new(x, p.y)),
        "right" => pick(horizontal_hits(poly, p.y), true).map_or(p, |x| Pt::new(x, p.y)),
        _ => p,
    }
}

fn corners(r: Rect) -> Vec<Pt> {
    vec![
        Pt::new(r.x, r.y),
        Pt::new(r.right(), r.y),
        Pt::new(r.right(), r.bottom()),
        Pt::new(r.x, r.bottom()),
    ]
}

fn rounded_polygon(r: Rect, rad: f64) -> Vec<Pt> {
    if rad <= 0.0 {
        return corners(r);
    }
    let Rect { x, y, w, h } = r;
    let p = Pt::new;
    let arcs = [
        (p(x + w - rad, y), p(x + w, y + rad)),
        (p(x + w, y + h - rad), p(x + w - rad, y + h)),
        (p(x + rad, y + h), p(x, y + h - rad)),
        (p(x, y + rad), p(x + rad, y)),
    ];
    let mut segs = Vec::new();
    let mut cur = p(x + rad, y);
    for (a, b) in arcs {
        if cur.dist(a) > 1e-9 {
            segs.push(PathSeg::Line { from: cur, to: a });
        }
        segs.extend(arc_to_cubics(a, rad, rad, 0.0, false, true, b));
        cur = b;
    }
    closed(flatten(&segs))
}

/// Drops the repeated first point of a flattened closed path.
fn closed(mut pts: Vec<Pt>) -> Vec<Pt> {
    if pts.len() > 1 && pts[0].dist(pts[pts.len() - 1]) <= 1e-9 {
        pts.pop();
    }
    pts
}
