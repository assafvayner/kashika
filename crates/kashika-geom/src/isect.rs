use crate::path::{PathSeg, flatten};
use crate::point::{Pt, TOLERANCE};
use crate::rect::Rect;

pub fn cross(a: Pt, b: Pt) -> f64 {
    a.x * b.y - a.y * b.x
}

pub fn dot(a: Pt, b: Pt) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Segments shorter than this are points.
const DEGENERATE: f64 = 1e-9;
/// Slack on the segment parameters, so endpoints that meet up to float error still hit.
const PARAM_EPS: f64 = 1e-12;

/// How two straight segments meet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Point(Pt),
    /// A collinear overlap, from one point to the other.
    Overlap(Pt, Pt),
}

/// The point on segment `ab` closest to `p`.
pub fn closest_on_seg(p: Pt, a: Pt, b: Pt) -> Pt {
    let d = b - a;
    let l2 = dot(d, d);
    if l2 <= DEGENERATE * DEGENERATE {
        return a;
    }
    a + d * (dot(p - a, d) / l2).clamp(0.0, 1.0)
}

pub fn dist_point_seg(p: Pt, a: Pt, b: Pt) -> f64 {
    p.dist(closest_on_seg(p, a, b))
}

/// Where segments `a0a1` and `b0b1` meet. Points within `eps` of the other segment count
/// as touching, and segments within `eps` of being collinear count as overlapping.
pub fn seg_hit(a0: Pt, a1: Pt, b0: Pt, b1: Pt, eps: f64) -> Option<Hit> {
    let eps = eps.max(0.0);
    let (da, db) = (a1 - a0, b1 - b0);
    let (la, lb) = (da.length(), db.length());
    if la <= DEGENERATE || lb <= DEGENERATE {
        let (p, q0, q1) = if la <= DEGENERATE {
            (a0, b0, b1)
        } else {
            (b0, a0, a1)
        };
        return (dist_point_seg(p, q0, q1) <= eps).then_some(Hit::Point(p));
    }
    let denom = cross(da, db);
    if denom.abs() <= 1e-9 * la * lb {
        if (cross(da, b0 - a0) / la).abs() > eps {
            return None;
        }
        let u = da * (1.0 / la);
        let (s0, s1) = (dot(b0 - a0, u), dot(b1 - a0, u));
        let lo = s0.min(s1).max(0.0);
        let hi = s0.max(s1).min(la);
        if hi < lo - eps {
            return None;
        }
        if hi - lo <= eps {
            return Some(Hit::Point(a0 + u * ((lo + hi) / 2.0).clamp(0.0, la)));
        }
        return Some(Hit::Overlap(a0 + u * lo, a0 + u * hi));
    }
    let t = cross(b0 - a0, db) / denom;
    let s = cross(b0 - a0, da) / denom;
    let in_range = |v: f64| (-PARAM_EPS..=1.0 + PARAM_EPS).contains(&v);
    if in_range(t) && in_range(s) {
        return Some(Hit::Point(a0 + da * t.clamp(0.0, 1.0)));
    }
    [(a0, b0, b1), (a1, b0, b1), (b0, a0, a1), (b1, a0, a1)]
        .into_iter()
        .find(|&(p, q0, q1)| dist_point_seg(p, q0, q1) <= eps)
        .map(|(p, _, _)| Hit::Point(p))
}

pub fn seg_seg_dist(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> f64 {
    if seg_hit(a0, a1, b0, b1, 0.0).is_some() {
        return 0.0;
    }
    dist_point_seg(a0, b0, b1)
        .min(dist_point_seg(a1, b0, b1))
        .min(dist_point_seg(b0, a0, a1))
        .min(dist_point_seg(b1, a0, a1))
}

pub fn rect_polygon(r: Rect) -> Vec<Pt> {
    vec![
        Pt::new(r.x, r.y),
        Pt::new(r.right(), r.y),
        Pt::new(r.right(), r.bottom()),
        Pt::new(r.x, r.bottom()),
    ]
}

/// The closed polygon's edges, including the one from the last vertex back to the first.
pub fn edges(poly: &[Pt]) -> impl Iterator<Item = (Pt, Pt)> + '_ {
    (0..poly.len()).map(move |i| (poly[i], poly[(i + 1) % poly.len()]))
}

/// Even-odd test. Points on the boundary may land either way; use `boundary_nearest` for those.
pub fn point_in_polygon(p: Pt, poly: &[Pt]) -> bool {
    let mut inside = false;
    for (a, b) in edges(poly) {
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside
}

/// Distance from `p` to the polygon's boundary, and the nearest boundary point.
pub fn boundary_nearest(p: Pt, poly: &[Pt]) -> (f64, Pt) {
    edges(poly)
        .map(|(a, b)| {
            let q = closest_on_seg(p, a, b);
            (p.dist(q), q)
        })
        .fold(
            (f64::INFINITY, p),
            |best, c| if c.0 < best.0 { c } else { best },
        )
}

/// Inside the polygon and farther than TOLERANCE from its boundary.
pub fn strictly_inside(p: Pt, poly: &[Pt]) -> bool {
    point_in_polygon(p, poly) && boundary_nearest(p, poly).0 > TOLERANCE
}

/// The parts of segment `ab` that lie strictly inside the polygon, in order from `a`.
pub fn inside_runs(a: Pt, b: Pt, poly: &[Pt]) -> Vec<(Pt, Pt)> {
    let d = b - a;
    let l2 = dot(d, d);
    if l2 <= DEGENERATE * DEGENERATE {
        return Vec::new();
    }
    let param = |p: Pt| (dot(p - a, d) / l2).clamp(0.0, 1.0);
    let mut ts = vec![0.0, 1.0];
    for (e0, e1) in edges(poly) {
        match seg_hit(a, b, e0, e1, 0.0) {
            Some(Hit::Point(p)) => ts.push(param(p)),
            Some(Hit::Overlap(p, q)) => {
                ts.push(param(p));
                ts.push(param(q));
            }
            None => {}
        }
    }
    ts.sort_by(f64::total_cmp);
    let mut runs: Vec<(Pt, Pt)> = Vec::new();
    for w in ts.windows(2) {
        let (p, q) = (a + d * w[0], a + d * w[1]);
        if p.dist(q) <= TOLERANCE || !strictly_inside(p.lerp(q, 0.5), poly) {
            continue;
        }
        match runs.last_mut() {
            Some(last) if last.1.dist(p) <= TOLERANCE => last.1 = q,
            _ => runs.push((p, q)),
        }
    }
    runs
}

/// Vertex average; inside any convex polygon.
pub fn centroid(poly: &[Pt]) -> Pt {
    let n = poly.len().max(1) as f64;
    poly.iter().fold(Pt::default(), |s, p| s + *p) * (1.0 / n)
}

/// Whether two polygons' interiors share area. Touching edges don't count. Either polygon
/// may be non-convex.
pub fn polygons_overlap(a: &[Pt], b: &[Pt]) -> bool {
    if a.len() < 3 || b.len() < 3 || !bounds_overlap(a, b) {
        return false;
    }
    edges(a).any(|(p, q)| !inside_runs(p, q, b).is_empty())
        || edges(b).any(|(p, q)| !inside_runs(p, q, a).is_empty())
        || interior_samples(a).any(|p| strictly_inside(p, b))
        || interior_samples(b).any(|p| strictly_inside(p, a))
}

fn bounds(poly: &[Pt]) -> (Pt, Pt) {
    poly.iter().fold(
        (
            Pt::new(f64::INFINITY, f64::INFINITY),
            Pt::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Pt::new(lo.x.min(p.x), lo.y.min(p.y)),
                Pt::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

fn bounds_overlap(a: &[Pt], b: &[Pt]) -> bool {
    let ((alo, ahi), (blo, bhi)) = (bounds(a), bounds(b));
    alo.x < bhi.x && blo.x < ahi.x && alo.y < bhi.y && blo.y < ahi.y
}

/// Points inside the polygon: the midpoint of every interior span of a horizontal scan
/// through the middle of each band between consecutive vertex heights.
fn interior_samples(poly: &[Pt]) -> impl Iterator<Item = Pt> + '_ {
    let mut ys: Vec<f64> = poly.iter().map(|p| p.y).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    ys.windows(2)
        .map(|w| (w[0] + w[1]) / 2.0)
        .collect::<Vec<_>>()
        .into_iter()
        .flat_map(move |y| {
            let mut xs: Vec<f64> = edges(poly)
                .filter(|(p, q)| (p.y > y) != (q.y > y))
                .map(|(p, q)| p.x + (y - p.y) * (q.x - p.x) / (q.y - p.y))
                .collect();
            xs.sort_by(f64::total_cmp);
            xs.chunks_exact(2)
                .map(|c| Pt::new((c[0] + c[1]) / 2.0, y))
                .collect::<Vec<_>>()
        })
}

/// Splits a path into straight pieces (curves flattened within FLATTEN_TOLERANCE),
/// each tagged with the index of the path segment it came from.
pub fn pieces(path: &[PathSeg]) -> Vec<(Pt, Pt, usize)> {
    let mut out = Vec::new();
    for (i, seg) in path.iter().enumerate() {
        let pts = flatten(std::slice::from_ref(seg));
        out.extend(pts.windows(2).map(|w| (w[0], w[1], i)));
    }
    out
}
