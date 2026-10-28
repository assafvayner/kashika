use kashika_diag::Diagnostic;
use kashika_geom::{Pt, Rect, TOLERANCE};
use kashika_model::Scene;
use serde::Serialize;

use crate::ctx::{fmt_num, round2, Ctx, LineGeom};

/// A closed range `lo..hi` on one axis.
pub type Interval = (f64, f64);

/// The axis that free values lie on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }
}

/// Values on `axis`, within `within`, where a straight run spanning `band` on the other axis
/// keeps `clearance` from every box, every line piece and every group border it runs along.
/// Ids in `skip` are ignored.
pub fn free_intervals(
    ctx: &Ctx,
    axis: Axis,
    band: Interval,
    within: Interval,
    skip: &[&str],
) -> Vec<Interval> {
    let c = ctx.scene.clearance;
    let (b0, b1) = (band.0.min(band.1), band.0.max(band.1));
    let mut blocked: Vec<Interval> = Vec::new();
    for g in ctx.groups() {
        if skip.contains(&g.shape.id.as_str()) {
            continue;
        }
        let r = g.shape.rect;
        let (across, sides) = match axis {
            Axis::Y => ((r.x, r.right()), [r.y, r.bottom()]),
            Axis::X => ((r.y, r.bottom()), [r.x, r.right()]),
        };
        if across.0 < b1 - TOLERANCE && across.1 > b0 + TOLERANCE {
            blocked.extend(sides.map(|v| (v - c, v + c)));
        }
    }
    let mut block = |r: Rect| {
        let (across, along) = match axis {
            Axis::Y => ((r.x, r.right()), (r.y, r.bottom())),
            Axis::X => ((r.y, r.bottom()), (r.x, r.right())),
        };
        if across.0 - c < b1 && across.1 + c > b0 {
            blocked.push((along.0 - c, along.1 + c));
        }
    };
    for s in ctx.boxes() {
        if !skip.contains(&s.shape.id.as_str()) {
            block(s.shape.rect);
        }
    }
    for l in &ctx.lines {
        if skip.contains(&l.line.id.as_str()) {
            continue;
        }
        for &(p, q, _) in &l.pieces {
            block(Rect::new(
                p.x.min(q.x),
                p.y.min(q.y),
                (p.x - q.x).abs(),
                (p.y - q.y).abs(),
            ));
        }
    }
    subtract(within, blocked)
}

fn subtract(within: Interval, mut blocked: Vec<Interval>) -> Vec<Interval> {
    blocked.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut cur = within.0;
    for (lo, hi) in blocked {
        if cur >= within.1 {
            break;
        }
        if lo > cur {
            out.push((cur, lo.min(within.1)));
        }
        cur = cur.max(hi);
    }
    if cur < within.1 {
        out.push((cur, within.1));
    }
    out.into_iter()
        .filter(|(lo, hi)| hi > lo)
        .map(|(lo, hi)| (round2(lo), round2(hi)))
        .collect()
}

/// Free lanes parallel to a straight horizontal or vertical piece.
#[derive(Debug, Clone, PartialEq)]
pub struct Free {
    pub axis: Axis,
    /// Where the piece runs now.
    pub at: f64,
    pub band: Interval,
    pub intervals: Vec<Interval>,
}

/// Free intervals around the piece `pq` across the whole canvas, or `None` when `pq` is diagonal.
pub fn free_around(ctx: &Ctx, p: Pt, q: Pt, skip: &[&str]) -> Option<Free> {
    let (w, h) = ctx.scene.size;
    let (axis, at, band, within) = if (p.y - q.y).abs() <= TOLERANCE {
        (Axis::Y, p.y, (p.x.min(q.x), p.x.max(q.x)), (0.0, h))
    } else if (p.x - q.x).abs() <= TOLERANCE {
        (Axis::X, p.x, (p.y.min(q.y), p.y.max(q.y)), (0.0, w))
    } else {
        return None;
    };
    Some(Free {
        axis,
        at,
        band,
        intervals: free_intervals(ctx, axis, band, within, skip),
    })
}

impl Free {
    pub fn fact(&self) -> serde_json::Value {
        serde_json::json!({
            "axis": self.axis.name(),
            "band": [round2(self.band.0), round2(self.band.1)],
            "intervals": self.intervals.iter().map(|(a, b)| [*a, *b]).collect::<Vec<_>>(),
        })
    }

    /// e.g. "free y-intervals across x 180..520: 60..104, 176..230"
    pub fn describe(&self) -> String {
        let other = match self.axis {
            Axis::X => "y",
            Axis::Y => "x",
        };
        let list = if self.intervals.is_empty() {
            "none".to_string()
        } else {
            self.intervals
                .iter()
                .map(|(a, b)| format!("{}..{}", fmt_num(*a), fmt_num(*b)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "free {}-intervals across {other} {}..{}: {list}",
            self.axis.name(),
            fmt_num(self.band.0),
            fmt_num(self.band.1)
        )
    }

    /// The free value closest to where the piece runs now.
    pub fn nearest(&self) -> Option<f64> {
        self.intervals
            .iter()
            .map(|&(lo, hi)| self.at.clamp(lo, hi))
            .min_by(|a, b| (a - self.at).abs().total_cmp(&(b - self.at).abs()))
    }
}

/// Adds the `free` fact and a "segment N runs at ..." help line for `path[k]` of `l`, when it is straight.
pub fn with_free(ctx: &Ctx, d: Diagnostic, l: &LineGeom, k: usize, skip: &[&str]) -> Diagnostic {
    match l
        .straight_ends(k)
        .and_then(|(p, q)| free_around(ctx, p, q, skip))
    {
        Some(f) => d.with_fact("free", f.fact()).with_help(format!(
            "segment {} runs at {}={}; {}",
            l.segment(k) + 1,
            f.axis.name(),
            fmt_num(f.at),
            f.describe()
        )),
        None => d,
    }
}

/// Free horizontal (`y`) and vertical (`x`) intervals inside `r`, for `inspect --corridors`.
pub fn corridors(scene: &Scene, r: Rect) -> (Vec<Interval>, Vec<Interval>) {
    let ctx = Ctx::new(scene);
    (
        free_intervals(&ctx, Axis::Y, (r.x, r.right()), (r.y, r.bottom()), &[]),
        free_intervals(&ctx, Axis::X, (r.y, r.bottom()), (r.x, r.right()), &[]),
    )
}
