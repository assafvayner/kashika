use crate::point::Pt;
use crate::rect::Rect;

/// Convex hull (Andrew's monotone chain), counter-clockwise in y-up terms, without
/// collinear points. Fewer than three distinct points come back as they are, sorted.
pub fn convex_hull(pts: &[Pt]) -> Vec<Pt> {
    let mut p: Vec<Pt> = pts
        .iter()
        .copied()
        .filter(|q| q.x.is_finite() && q.y.is_finite())
        .collect();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: Pt, a: Pt, b: Pt| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Pt> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0.0 {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<Pt> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0.0 {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The y values where the vertical line `x` meets the edges of the closed polygon `poly`.
pub fn vertical_hits(poly: &[Pt], x: f64) -> Vec<f64> {
    hits(poly, x, |p| (p.x, p.y))
}

/// The x values where the horizontal line `y` meets the edges of the closed polygon `poly`.
pub fn horizontal_hits(poly: &[Pt], y: f64) -> Vec<f64> {
    hits(poly, y, |p| (p.y, p.x))
}

/// Sorted, with hits closer than 1e-9 merged, so a cut through a vertex reports it once.
/// `axes` returns (coordinate along the cut, coordinate reported) for a point.
fn hits(poly: &[Pt], at: f64, axes: impl Fn(Pt) -> (f64, f64)) -> Vec<f64> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a0, a1) = axes(poly[i]);
        let (b0, b1) = axes(poly[(i + 1) % poly.len()]);
        if at < a0.min(b0) - 1e-9 || at > a0.max(b0) + 1e-9 {
            continue;
        }
        if (b0 - a0).abs() < 1e-12 {
            out.push(a1);
            out.push(b1);
        } else {
            out.push(a1 + (b1 - a1) * (at - a0) / (b0 - a0));
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    out
}

/// The part of the convex polygon `poly` inside `r` (Sutherland-Hodgman). Empty when they
/// do not meet.
pub fn clip_to_rect(poly: &[Pt], r: Rect) -> Vec<Pt> {
    let coord = |p: Pt, x: bool| if x { p.x } else { p.y };
    let planes = [
        (true, r.x, 1.0),
        (true, r.right(), -1.0),
        (false, r.y, 1.0),
        (false, r.bottom(), -1.0),
    ];
    let mut out = poly.to_vec();
    for (x, at, sign) in planes {
        let inside = |p: Pt| (coord(p, x) - at) * sign >= 0.0;
        let input = std::mem::take(&mut out);
        for (i, &b) in input.iter().enumerate() {
            let a = input[(i + input.len() - 1) % input.len()];
            if inside(b) != inside(a) {
                let t = (at - coord(a, x)) / (coord(b, x) - coord(a, x));
                out.push(a.lerp(b, t));
            }
            if inside(b) {
                out.push(b);
            }
        }
    }
    out
}
