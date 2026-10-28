use kashika_geom::{boundary_nearest, Pt, TOLERANCE};
use kashika_model::Marker;

use crate::ctx::{side_anchor, Ctx};

/// A line endpoint that sits on a shape's outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub shape: String,
    pub side: &'static str,
    pub t: f64,
    pub at: Pt,
    pub line: String,
    /// `start` or `end`.
    pub end: &'static str,
    /// Whether the line draws a marker at this end.
    pub marker: bool,
}

/// Every endpoint on an outline, in shape order then line order. Endpoints at junctions are skipped.
pub fn ports(ctx: &Ctx) -> Vec<Port> {
    let mut out = Vec::new();
    for s in &ctx.shapes {
        for l in &ctx.lines {
            for (p, end, marker) in [(l.start, "start", l.line.start), (l.end, "end", l.line.end)] {
                if ctx.is_junction(p) || boundary_nearest(p, &s.poly).0 > TOLERANCE {
                    continue;
                }
                let (side, t) = side_anchor(s.shape, p);
                out.push(Port {
                    shape: s.shape.id.clone(),
                    side,
                    t,
                    at: p,
                    line: l.line.id.clone(),
                    end,
                    marker: marker != Marker::None,
                });
            }
        }
    }
    out
}
