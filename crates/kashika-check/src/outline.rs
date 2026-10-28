use std::f64::consts::{FRAC_PI_2, PI};

use kashika_geom::{rect_polygon, Pt, Rect};
use kashika_model::{Outline, Shape};

/// The outline the checks use, as a closed polygon in canvas coordinates: the flattened
/// form outline for built-in, custom and SVG shapes (box or hull), the box for images.
pub fn polygon(s: &Shape) -> Vec<Pt> {
    if let Some(f) = s.form.as_ref().filter(|f| f.outline.len() >= 3) {
        return f.outline.clone();
    }
    let r = s.corner_radius();
    match s.outline {
        Outline::Rounded if r > 0.0 => rounded(s.rect, r),
        _ => rect_polygon(s.rect),
    }
}

/// Quarter arcs at each corner, clockwise from the top-left, flattened within 0.25px.
fn rounded(b: Rect, r: f64) -> Vec<Pt> {
    let steps = ((FRAC_PI_2 * (r / 2.0).sqrt()).ceil() as usize).max(2);
    let corners = [
        (b.x + r, b.y + r, PI),
        (b.right() - r, b.y + r, 1.5 * PI),
        (b.right() - r, b.bottom() - r, 0.0),
        (b.x + r, b.bottom() - r, FRAC_PI_2),
    ];
    let mut pts = Vec::new();
    for (cx, cy, a0) in corners {
        for k in 0..=steps {
            let a = a0 + FRAC_PI_2 * k as f64 / steps as f64;
            pts.push(Pt::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}
