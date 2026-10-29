use kashika_diag::{Diagnostic, Pending};
use kashika_geom::{Pt, Rect, TOLERANCE, fmt_num, round2};
use kashika_model::Scene;
use serde::Serialize;

use crate::ctx::{Ctx, LineGeom};

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
/// keeps `clearance` from every box, image, line piece, junction, text, line label and label
/// under a `text-area=below` shape, and from every group border it runs along. Ids in `skip`
/// are ignored, along with the labels of skipped lines and shapes.
pub fn free_intervals(
    ctx: &Ctx,
    axis: Axis,
    band: Interval,
    within: Interval,
    skip: &[&str],
) -> Vec<Interval> {
    let c = ctx.scene.clearance;
    let (b0, b1) = (band.0.min(band.1), band.0.max(band.1));
    let big = f64::MAX / 4.0;
    let strip = match axis {
        Axis::Y => Rect::new(b0, -big, b1 - b0, 2.0 * big),
        Axis::X => Rect::new(-big, b0, 2.0 * big, b1 - b0),
    };
    let mut blocked: Vec<Interval> = Vec::new();
    for g in ctx.groups_near(strip, 0.0) {
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
    for s in ctx.boxes_near(strip, c) {
        if !skip.contains(&s.shape.id.as_str()) {
            block(s.shape.rect);
        }
    }
    for l in ctx.lines_near(strip, c).into_iter().map(|i| &ctx.lines[i]) {
        if skip.contains(&l.line.id.as_str()) {
            continue;
        }
        for &(p, q, _) in &l.pieces {
            block(Rect::from_points(p, q));
        }
    }
    for m in ctx.marks_near(strip, c) {
        if !skip.contains(&m.owner) {
            block(m.rect);
        }
    }
    subtract(within, blocked)
}

pub(crate) fn subtract(within: Interval, mut blocked: Vec<Interval>) -> Vec<Interval> {
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
        .map(|(lo, hi)| (round2(lo), round2(hi)))
        .filter(|(lo, hi)| hi > lo)
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

/// Help that is costly to work out, so it is filled in only for diagnostics that are shown.
/// A diagnostic carries its entries in `Diagnostic::pending`.
#[derive(Debug, Clone, PartialEq)]
pub struct Deferred {
    /// Where in `help` its lines go.
    pub help_at: usize,
    pub kind: DeferredKind,
    /// The straight run the free lanes are around.
    pub from: [f64; 2],
    pub to: [f64; 2],
    /// Ids ignored when looking for free lanes.
    pub skip: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredKind {
    /// "segment N runs at ..." help and the `free` fact, for 0-based authored segment N.
    Segment(usize),
    /// "nearest free lane" and free-interval help, with the `lane` and `free` facts.
    Lane,
}

/// The help and facts still to be worked out for `d`, in the order they were deferred.
pub fn deferred(d: &Diagnostic) -> &[Deferred] {
    d.pending.get::<Vec<Deferred>>().map_or(&[], Vec::as_slice)
}

/// Defers help of `kind` around the straight run `from`..`to` to `resolve`; its lines go after
/// the help `d` has so far.
pub fn with_deferred(
    mut d: Diagnostic,
    kind: DeferredKind,
    from: Pt,
    to: Pt,
    skip: &[&str],
) -> Diagnostic {
    let mut all: Vec<Deferred> = d.pending.take().unwrap_or_default();
    all.push(Deferred {
        help_at: d.help.len(),
        kind,
        from: [from.x, from.y],
        to: [to.x, to.y],
        skip: skip.iter().map(|s| s.to_string()).collect(),
    });
    d.pending = Pending::new(all);
    d
}

/// Defers the `free` fact and a "segment N runs at ..." help line for `path[k]` of `l`, when it
/// is straight, to `resolve`.
pub fn with_free(d: Diagnostic, l: &LineGeom, k: usize, skip: &[&str]) -> Diagnostic {
    match l.straight_ends(k) {
        Some((p, q)) => with_deferred(d, DeferredKind::Segment(l.segment(k)), p, q, skip),
        None => d,
    }
}

/// Works out the help and facts `d` deferred.
pub fn resolve(ctx: &Ctx, d: &mut Diagnostic) {
    let all: Vec<Deferred> = d.pending.take().unwrap_or_default();
    for x in all.into_iter().rev() {
        let skip: Vec<&str> = x.skip.iter().map(String::as_str).collect();
        let pt = |a: [f64; 2]| Pt::new(a[0], a[1]);
        let Some(f) = free_around(ctx, pt(x.from), pt(x.to), &skip) else {
            continue;
        };
        let mut help = Vec::new();
        match x.kind {
            DeferredKind::Segment(n) => help.push(format!(
                "segment {} runs at {}={}; {}",
                n + 1,
                f.axis.name(),
                fmt_num(f.at),
                f.describe()
            )),
            DeferredKind::Lane => {
                if let Some(lane) = f.nearest() {
                    d.facts.insert(
                        "lane".into(),
                        serde_json::json!({"axis": f.axis.name(), "at": lane}),
                    );
                    help.push(format!(
                        "nearest free lane: {}={}",
                        f.axis.name(),
                        fmt_num(lane)
                    ));
                }
                help.push(f.describe());
            }
        }
        d.facts.insert("free".into(), f.fact());
        let at = x.help_at.min(d.help.len());
        d.help.splice(at..at, help);
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
