use kashika_geom::{Rect, flatten};

use super::Resolver;
use crate::scene::{Junction, Line, Scene, Shape, Text};

/// Radius of a junction's `dot`.
const DOT_RADIUS: f64 = 3.0;

impl Resolver<'_> {
    /// Sets each `auto` canvas axis to the far edge of everything drawn, plus `margin`.
    pub(super) fn fit_canvas(&self, scene: &mut Scene) {
        let (fit_w, fit_h) = self.canvas_fit;
        scene.fitted = self.canvas_fit;
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

/// The largest x and y that anything painted reaches.
fn extent(scene: &Scene) -> (f64, f64) {
    let mut far = Reach::default();
    for s in &scene.shapes {
        far.shape(s);
    }
    for j in &scene.junctions {
        far.junction(j);
    }
    for t in &scene.texts {
        far.text(t);
    }
    for l in &scene.lines {
        far.line(l);
    }
    far.far()
}

/// The largest x and y painted items reach: shapes with their strokes, text and icons
/// (labels under a shape included), junction dots, texts, and lines with their strokes,
/// markers and labels.
pub(super) struct Reach(f64, f64);

impl Default for Reach {
    fn default() -> Self {
        Reach(f64::NEG_INFINITY, f64::NEG_INFINITY)
    }
}

impl Reach {
    pub(super) fn far(&self) -> (f64, f64) {
        (self.0, self.1)
    }

    fn rect(&mut self, r: Rect, pad: f64) {
        self.0 = self.0.max(r.right() + pad);
        self.1 = self.1.max(r.bottom() + pad);
    }

    pub(super) fn shape(&mut self, s: &Shape) {
        self.rect(s.rect, s.style.stroke_width / 2.0);
        for t in &s.text {
            self.rect(t.rect, 0.0);
        }
        if let Some(icon) = &s.icon {
            self.rect(icon.rect, 0.0);
        }
    }

    pub(super) fn junction(&mut self, j: &Junction) {
        let r = if j.dot { DOT_RADIUS } else { 0.0 };
        self.rect(Rect::new(j.at.x, j.at.y, 0.0, 0.0), r);
    }

    pub(super) fn text(&mut self, t: &Text) {
        self.rect(t.rect, 0.0);
    }

    pub(super) fn line(&mut self, l: &Line) {
        let pad = (l.style.stroke_width / 2.0)
            .max(l.start.half_width(l.style.stroke_width))
            .max(l.end.half_width(l.style.stroke_width));
        for p in flatten(&l.path) {
            self.rect(Rect::new(p.x, p.y, 0.0, 0.0), pad);
        }
        for lab in &l.labels {
            self.rect(lab.rect, 0.0);
        }
    }
}
