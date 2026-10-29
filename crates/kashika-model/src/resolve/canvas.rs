use kashika_geom::{Rect, flatten};

use super::Resolver;
use crate::scene::Scene;

/// Radius of a junction's `dot`.
const DOT_RADIUS: f64 = 3.0;

impl Resolver<'_> {
    /// Sets each `auto` canvas axis to the far edge of everything drawn, plus `margin`.
    pub(super) fn fit_canvas(&self, scene: &mut Scene) {
        let (fit_w, fit_h) = self.canvas_fit;
        if !fit_w && !fit_h {
            return;
        }
        let (w, h) = extent(scene);
        if fit_w {
            scene.size.0 = w.max(0.0).ceil() + self.margin;
        }
        if fit_h {
            scene.size.1 = h.max(0.0).ceil() + self.margin;
        }
    }
}

/// The largest x and y that anything painted reaches: shapes with their strokes, text and
/// icons (labels under a shape included), junction dots, texts, and lines with their
/// strokes, markers and labels.
fn extent(scene: &Scene) -> (f64, f64) {
    let mut far = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut reach = |r: Rect, pad: f64| {
        far.0 = far.0.max(r.right() + pad);
        far.1 = far.1.max(r.bottom() + pad);
    };
    for s in &scene.shapes {
        reach(s.rect, s.style.stroke_width / 2.0);
        for t in &s.text {
            reach(t.rect, 0.0);
        }
        if let Some(icon) = &s.icon {
            reach(icon.rect, 0.0);
        }
    }
    for j in &scene.junctions {
        let r = if j.dot { DOT_RADIUS } else { 0.0 };
        reach(Rect::new(j.at.x, j.at.y, 0.0, 0.0), r);
    }
    for t in &scene.texts {
        reach(t.rect, 0.0);
    }
    for l in &scene.lines {
        let pad = (l.style.stroke_width / 2.0)
            .max(l.start.half_width())
            .max(l.end.half_width());
        for p in flatten(&l.path) {
            reach(Rect::new(p.x, p.y, 0.0, 0.0), pad);
        }
        for lab in &l.labels {
            reach(lab.rect, 0.0);
        }
    }
    far
}
