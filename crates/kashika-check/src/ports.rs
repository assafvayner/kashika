use kashika_geom::{Pt, TOLERANCE, boundary_nearest};
use kashika_model::Marker;

use crate::ctx::{Ctx, outline_anchor_toward};

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
    /// How far that marker reaches to either side of the line; 0 without one.
    pub marker_half_width: f64,
}

/// Every endpoint on an outline, in shape order then line order. Endpoints at junctions are skipped.
pub fn ports(ctx: &Ctx) -> Vec<Port> {
    let mut out = Vec::new();
    for (i, s) in ctx.shapes.iter().enumerate() {
        for l in ctx
            .lines_near(ctx.shape_boxes[i], TOLERANCE)
            .into_iter()
            .map(|j| &ctx.lines[j])
        {
            let out_start = l.line.path.first().map(|seg| seg.start_dir());
            let out_end = l.line.path.last().map(|seg| seg.end_dir() * -1.0);
            for (p, end, marker, dir) in [
                (l.start, "start", l.line.start, out_start),
                (l.end, "end", l.line.end, out_end),
            ] {
                if ctx.is_junction(p) || boundary_nearest(p, &s.poly).0 > TOLERANCE {
                    continue;
                }
                let (side, t) = outline_anchor_toward(s.shape, p, dir);
                out.push(Port {
                    shape: s.shape.id.clone(),
                    side,
                    t,
                    at: p,
                    line: l.line.id.clone(),
                    end,
                    marker: marker != Marker::None,
                    marker_half_width: marker.half_width(l.line.style.stroke_width),
                });
            }
        }
    }
    out
}
