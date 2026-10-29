use std::cmp::Ordering;

use kashika_check::ctx::Ctx;
use kashika_check::free::{Axis, free_intervals};
use kashika_geom::{
    Hit, Pt, Rect, TOLERANCE, boundary_nearest, dot, inside_runs, point_in_polygon, seg_hit,
};
use kashika_model::{Scene, ShapeKind};

use crate::{Endpoint, SQUARE, Style};
use kashika_geom::fmt_num as num;

/// Lanes per axis that detours pick from: those nearest either endpoint.
pub(crate) const DETOUR_LANES: usize = 16;
/// One-lane plans scored with `kashika check`: the best of a geometric pre-pass.
pub(crate) const MAX_LANE_PLANS: usize = 32;
/// Detours scored with `kashika check`: the best of a geometric pre-pass.
pub(crate) const MAX_DETOURS: usize = 32;

/// An `x` or `y` target: relative to a box corner where possible, otherwise a literal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Lane {
    pub(crate) value: f64,
    pub(crate) expr: Option<String>,
}

/// A `curve via` control point.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ctrl {
    pub(crate) at: Pt,
    pub(crate) expr: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Hop {
    /// Straight to the end point.
    To,
    /// `hv <end>`
    Hv,
    /// `vh <end>`
    Vh,
    X(Lane),
    Y(Lane),
    Curve {
        left: bool,
        amount: f64,
    },
    Via(Ctrl, Ctrl),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plan {
    pub(crate) hops: Vec<Hop>,
    /// The lane the path is routed along: `("x", v)` when a vertical segment runs at x=v. For
    /// a detour, its second lane.
    pub(crate) lane: Option<(&'static str, f64)>,
}

/// How a plan's lanes are anchored, lower first: lanes on a box the path neither starts nor
/// ends at, which move with a bystander, then literal lanes, which move with nothing.
pub(crate) type LaneAnchors = (usize, usize);

impl Plan {
    pub(crate) fn lane_anchors(&self, a: &Endpoint, b: &Endpoint) -> LaneAnchors {
        let owns = |id: &str| [a, b].iter().any(|e| e.owner.as_deref() == Some(id));
        self.hops
            .iter()
            .filter_map(|h| match h {
                Hop::X(l) | Hop::Y(l) => Some(l),
                _ => None,
            })
            .fold((0, 0), |(bystander, literal), l| match &l.expr {
                Some(e) if owns(e.split('.').next().unwrap_or_default()) => (bystander, literal),
                Some(_) => (bystander + 1, literal),
                None => (bystander, literal + 1),
            })
    }
}

/// The plans worth scoring, without two that draw the same path. Lines in `skip` aren't
/// obstacles when picking lanes.
pub(crate) fn generate(
    ctx: &Ctx,
    from: &Endpoint,
    to: &Endpoint,
    style: Style,
    skip: &[&str],
) -> Vec<Plan> {
    let plans = match style {
        Style::Bezier => curves(from, to),
        Style::Corner | Style::Arc => orthogonal(ctx, from, to, skip),
    };
    let mut seen: Vec<Vec<(i64, i64)>> = Vec::new();
    plans
        .into_iter()
        .filter(|p| match corners(from.at, to.at, p) {
            Some(pts) => {
                let k = key(pts);
                let new = !seen.contains(&k);
                if new {
                    seen.push(k);
                }
                new
            }
            None => true,
        })
        .collect()
}

/// The corners of an orthogonal plan from `a` to `b`; `None` for curves.
pub(crate) fn corners(a: Pt, b: Pt, plan: &Plan) -> Option<Vec<Pt>> {
    let mut pts = vec![a];
    for hop in &plan.hops {
        let at = pts[pts.len() - 1];
        match hop {
            Hop::To => pts.push(b),
            Hop::Hv => pts.extend([Pt::new(b.x, at.y), b]),
            Hop::Vh => pts.extend([Pt::new(at.x, b.y), b]),
            Hop::X(l) => pts.push(Pt::new(l.value, at.y)),
            Hop::Y(l) => pts.push(Pt::new(at.x, l.value)),
            Hop::Curve { .. } | Hop::Via(..) => return None,
        }
    }
    Some(pts)
}

/// Whether a path through `pts` has a zero-length segment or one that reverses the one before.
pub(crate) fn doubles_back(pts: &[Pt]) -> bool {
    pts.windows(2).any(|w| w[0].dist(w[1]) <= TOLERANCE)
        || pts
            .windows(3)
            .any(|w| dot((w[1] - w[0]).norm(), (w[2] - w[1]).norm()) < -SQUARE)
}

/// Points at 0.1px, without repeats or points in the middle of a straight run, so paths that
/// draw the same shape get the same key.
pub(crate) fn key(pts: impl IntoIterator<Item = Pt>) -> Vec<(i64, i64)> {
    let mut key: Vec<(i64, i64)> = Vec::new();
    for p in pts {
        let q = ((p.x * 10.0).round() as i64, (p.y * 10.0).round() as i64);
        if key.last() == Some(&q) {
            continue;
        }
        if let [.., a, b] = key[..] {
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (q.0 - b.0, q.1 - b.1));
            if u.0 * v.1 == u.1 * v.0 && u.0 * v.0 + u.1 * v.1 > 0 {
                key.pop();
            }
        }
        key.push(q);
    }
    key
}

/// The straight path, `hv` and `vh`, the best [`MAX_LANE_PLANS`] one-lane plans and the best
/// detours. Plans that double back are dropped.
fn orthogonal(ctx: &Ctx, a: &Endpoint, b: &Endpoint, skip: &[&str]) -> Vec<Plan> {
    let mut simple = Vec::new();
    if (a.at.x - b.at.x).abs() <= 0.5 || (a.at.y - b.at.y).abs() <= 0.5 {
        simple.push(vec![Hop::To]);
    }
    simple.push(vec![Hop::Hv]);
    simple.push(vec![Hop::Vh]);
    let mut out: Vec<Plan> = simple
        .into_iter()
        .map(|hops| Plan { hops, lane: None })
        .filter(|p| !corners(a.at, b.at, p).is_some_and(|pts| doubles_back(&pts)))
        .collect();
    let field = Field::new(ctx, a, b, skip);
    let mut found = Vec::new();
    for (axis, cross) in [(Axis::X, Hop::Vh), (Axis::Y, Hop::Hv)] {
        for lane in lane_pool(ctx, a, b, axis, skip) {
            let hops = vec![hop(axis.name(), &lane), cross.clone()];
            field.consider(&mut found, hops, (axis.name(), lane.value));
        }
    }
    out.extend(best(found, MAX_LANE_PLANS));
    out.extend(detours(ctx, &field, a, b, skip));
    out
}

/// The `n` plans with the best estimates, in that order; ties keep the order they were found.
fn best(mut found: Vec<(Estimate, Plan)>, n: usize) -> impl Iterator<Item = Plan> {
    found.sort_by_key(|f| f.0);
    found.into_iter().take(n).map(|(_, plan)| plan)
}

/// Three- and four-elbow plans through two or three lanes: `x -> y -> hv` and `y -> x -> vh`
/// end on the other axis than they start, `x -> y -> x -> vh` and `y -> x -> y -> hv` on the
/// same one. A geometric pre-pass drops paths that enter a box, double back, or leave or enter
/// a side anchor the wrong way, and keeps the [`MAX_DETOURS`] best by estimated crossings,
/// near-misses, segments and length.
fn detours(ctx: &Ctx, field: &Field, a: &Endpoint, b: &Endpoint, skip: &[&str]) -> Vec<Plan> {
    let xs = detour_lanes(ctx, a, b, Axis::X, skip);
    let ys = detour_lanes(ctx, a, b, Axis::Y, skip);
    let mut found: Vec<(Estimate, Plan)> = Vec::new();
    for (lead, back) in [("x", "y"), ("y", "x")] {
        let (outer, inner) = if lead == "x" { (&xs, &ys) } else { (&ys, &xs) };
        let (cross, along) = if lead == "x" {
            (Hop::Hv, Hop::Vh)
        } else {
            (Hop::Vh, Hop::Hv)
        };
        for l1 in outer {
            for l2 in inner {
                let lane = (back, l2.value);
                field.consider(
                    &mut found,
                    vec![hop(lead, l1), hop(back, l2), cross.clone()],
                    lane,
                );
                for l3 in outer {
                    field.consider(
                        &mut found,
                        vec![hop(lead, l1), hop(back, l2), hop(lead, l3), along.clone()],
                        lane,
                    );
                }
            }
        }
    }
    best(found, MAX_DETOURS).collect()
}

fn hop(axis: &str, lane: &Lane) -> Hop {
    if axis == "x" {
        Hop::X(lane.clone())
    } else {
        Hop::Y(lane.clone())
    }
}

/// Lanes on one axis: those of [`lanes`], plus the middle of each free interval (as
/// `inspect --corridors` reports between the endpoints) that holds none of them other than a
/// group's, at least `clearance` inside the canvas.
fn lane_pool(ctx: &Ctx, a: &Endpoint, b: &Endpoint, axis: Axis, skip: &[&str]) -> Vec<Lane> {
    let scene = ctx.scene;
    let (band, limit) = match axis {
        Axis::X => ((a.at.y, b.at.y), scene.size.0),
        Axis::Y => ((a.at.x, b.at.x), scene.size.1),
    };
    let margin = scene.clearance.max(TOLERANCE);
    let on_group = |l: &Lane| {
        l.expr.as_deref().is_some_and(|e| {
            let id = e.split('.').next().unwrap_or_default();
            scene
                .shapes
                .iter()
                .any(|s| s.id == id && s.kind == ShapeKind::Group)
        })
    };
    let mut pool = lanes(scene, a, b, axis);
    for (lo, hi) in free_intervals(ctx, axis, band, (0.0, limit), skip) {
        if pool
            .iter()
            .any(|l| lo <= l.value && l.value <= hi && !on_group(l))
        {
            continue;
        }
        let v = snap(scene, (lo + hi) / 2.0);
        let v = if lo <= v && v <= hi {
            v
        } else {
            (lo + hi) / 2.0
        };
        if v >= margin && v <= limit - margin {
            pool.push(Lane {
                value: v,
                expr: None,
            });
        }
    }
    pool
}

/// Lanes for detours on one axis: the [`lane_pool`] lanes at least `clearance` inside the
/// canvas. The [`DETOUR_LANES`] nearest either endpoint are kept.
fn detour_lanes(ctx: &Ctx, a: &Endpoint, b: &Endpoint, axis: Axis, skip: &[&str]) -> Vec<Lane> {
    let scene = ctx.scene;
    let (pa, pb, limit) = match axis {
        Axis::X => (a.at.x, b.at.x, scene.size.0),
        Axis::Y => (a.at.y, b.at.y, scene.size.1),
    };
    let margin = scene.clearance.max(TOLERANCE);
    let mut pool = lane_pool(ctx, a, b, axis, skip);
    pool.retain(|l| l.value >= margin && l.value <= limit - margin);
    pool.sort_by_key(|l| l.expr.is_none());
    let mut out: Vec<Lane> = Vec::new();
    for l in pool {
        if !out.iter().any(|o| (o.value - l.value).abs() <= 0.5) {
            out.push(l);
        }
    }
    let away = |v: f64| (v - pa).abs().min((v - pb).abs());
    out.sort_by(|p, q| away(p.value).total_cmp(&away(q.value)));
    out.truncate(DETOUR_LANES);
    out
}

fn snap(scene: &Scene, v: f64) -> f64 {
    match scene.grid {
        Some(grid) if grid > 0.0 => (v / grid).round() * grid,
        _ => v.round(),
    }
}

/// Estimated crossings and group transits, near-misses and segments, then length: the order
/// `route` ranks by.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Estimate(usize, usize, usize, f64);

impl Eq for Estimate {}

impl PartialOrd for Estimate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Estimate {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0, self.1, self.2)
            .cmp(&(other.0, other.1, other.2))
            .then(self.3.total_cmp(&other.3))
    }
}

/// The obstacles a plan's corners are measured against.
struct Field<'a> {
    boxes: Vec<(&'a str, Rect, &'a [Pt])>,
    /// Groups that hold neither endpoint: passing through one is a transit.
    groups: Vec<&'a [Pt]>,
    pieces: Vec<(Pt, Pt)>,
    starts: Vec<&'a str>,
    ends: Vec<&'a str>,
    from: &'a Endpoint,
    to: &'a Endpoint,
    clearance: f64,
}

impl<'a> Field<'a> {
    fn new(ctx: &'a Ctx, from: &'a Endpoint, to: &'a Endpoint, skip: &[&str]) -> Self {
        Field {
            boxes: ctx
                .boxes()
                .map(|s| (s.shape.id.as_str(), s.shape.rect, s.poly.as_slice()))
                .collect(),
            groups: ctx
                .groups()
                .map(|g| g.poly.as_slice())
                .filter(|poly| {
                    ![from.at, to.at].iter().any(|&p| {
                        point_in_polygon(p, poly) || boundary_nearest(p, poly).0 <= TOLERANCE
                    })
                })
                .collect(),
            pieces: ctx
                .lines
                .iter()
                .filter(|l| !skip.contains(&l.line.id.as_str()))
                .flat_map(|l| l.pieces.iter().map(|&(p, q, _)| (p, q)))
                .collect(),
            starts: ctx.attached(from.at),
            ends: ctx.attached(to.at),
            from,
            to,
            clearance: ctx.scene.clearance,
        }
    }

    /// Adds `hops` to `found` with its estimate, unless [`Field::estimate`] rules it out.
    fn consider(
        &self,
        found: &mut Vec<(Estimate, Plan)>,
        hops: Vec<Hop>,
        lane: (&'static str, f64),
    ) {
        let plan = Plan {
            hops,
            lane: Some(lane),
        };
        let pts = corners(self.from.at, self.to.at, &plan).expect("orthogonal hops");
        if let Some(e) = self.estimate(&pts) {
            found.push((e, plan));
        }
    }

    /// `None` when the path doubles back, leaves or enters a side anchor the wrong way, or
    /// enters a box. The first and last segments may touch the boxes they attach to. Each
    /// group the path passes through counts as a crossing.
    fn estimate(&self, pts: &[Pt]) -> Option<Estimate> {
        let n = pts.len() - 1;
        if doubles_back(pts) {
            return None;
        }
        let dir = |i: usize| (pts[i + 1] - pts[i]).norm();
        if self
            .from
            .side
            .is_some_and(|s| dot(dir(0), s.outward()) <= SQUARE)
            || self
                .to
                .side
                .is_some_and(|s| dot(dir(n - 1), s.outward()) >= -SQUARE)
        {
            return None;
        }
        let c = self.clearance;
        let (mut crossings, mut near, mut length) = (0, 0, 0.0);
        for (i, w) in pts.windows(2).enumerate() {
            let (p, q) = (w[0], w[1]);
            length += p.dist(q);
            let (lo, hi) = (
                Pt::new(p.x.min(q.x), p.y.min(q.y)),
                Pt::new(p.x.max(q.x), p.y.max(q.y)),
            );
            for &(id, r, poly) in &self.boxes {
                if (i == 0 && self.starts.contains(&id)) || (i == n - 1 && self.ends.contains(&id))
                {
                    continue;
                }
                if hi.x <= r.x - c
                    || lo.x >= r.right() + c
                    || hi.y <= r.y - c
                    || lo.y >= r.bottom() + c
                {
                    continue;
                }
                if !inside_runs(p, q, poly).is_empty() {
                    return None;
                }
                near += 1;
            }
            for &(u, v) in &self.pieces {
                match seg_hit(p, q, u, v, 0.0) {
                    Some(Hit::Point(x))
                        if x.dist(self.from.at) > TOLERANCE && x.dist(self.to.at) > TOLERANCE =>
                    {
                        crossings += 1
                    }
                    Some(Hit::Overlap(..)) => near += 1,
                    _ => {}
                }
            }
        }
        crossings += self
            .groups
            .iter()
            .filter(|poly| {
                pts.windows(2)
                    .any(|w| !inside_runs(w[0], w[1], poly).is_empty())
            })
            .count();
        Some(Estimate(crossings, near, n, length))
    }
}

/// Lane positions on one axis, inside the canvas: the midpoint between the endpoints, and
/// `2 × clearance` outside each box or group edge, written relative to its corner so the lane
/// follows it. Of lanes within 0.5px of each other only one is kept: one on a box the
/// path starts or ends at, otherwise the box nearest the endpoints, before the midpoint. They
/// come in that order of preference.
fn lanes(scene: &Scene, a: &Endpoint, b: &Endpoint, axis: Axis) -> Vec<Lane> {
    let g = 2.0 * scene.clearance;
    let gap = num(g);
    let (pa, pb, limit) = match axis {
        Axis::X => (a.at.x, b.at.x, scene.size.0),
        Axis::Y => (a.at.y, b.at.y, scene.size.1),
    };
    let ends = Rect::from_points(a.at, b.at);
    let owns = |id: &str| [a, b].iter().any(|e| e.owner.as_deref() == Some(id));
    let mut pool: Vec<(Rank, Lane)> = vec![(
        Rank(true, true, 0.0),
        Lane {
            value: snap(scene, (pa + pb) / 2.0),
            expr: None,
        },
    )];
    for s in scene.shapes.iter().filter(|s| s.layer.is_normal()) {
        let (r, id) = (s.rect, &s.id);
        let rank = Rank(false, !owns(id), gap_between(r, ends));
        let (before, after) = match axis {
            Axis::X => (
                (r.x - g, format!("{id}.tl+(-{gap},0)")),
                (r.right() + g, format!("{id}.tr+({gap},0)")),
            ),
            Axis::Y => (
                (r.y - g, format!("{id}.tl+(0,-{gap})")),
                (r.bottom() + g, format!("{id}.bl+(0,{gap})")),
            ),
        };
        for (value, expr) in [before, after] {
            pool.push((
                rank,
                Lane {
                    value,
                    expr: Some(expr),
                },
            ));
        }
    }
    pool.retain(|(_, l)| l.value > 0.0 && l.value < limit);
    pool.sort_by(|p, q| p.1.value.total_cmp(&q.1.value));
    let mut kept: Vec<(Rank, Lane)> = Vec::new();
    let mut first = f64::NEG_INFINITY;
    for (rank, lane) in pool {
        match kept.last_mut() {
            Some(last) if lane.value - first <= 0.5 => {
                if rank < last.0 {
                    *last = (rank, lane);
                }
            }
            _ => {
                first = lane.value;
                kept.push((rank, lane));
            }
        }
    }
    kept.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap_or(Ordering::Equal));
    kept.into_iter().map(|(_, l)| l).collect()
}

/// How a lane ranks among lanes at the same position, lower first: whether it is a literal,
/// whether it is on a box other than an endpoint's, and that box's distance from the endpoints.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Rank(bool, bool, f64);

/// The distance between two rects, 0 when they touch or overlap.
fn gap_between(r: Rect, s: Rect) -> f64 {
    let dx = (s.x - r.right()).max(r.x - s.right()).max(0.0);
    let dy = (s.y - r.bottom()).max(r.y - s.bottom()).max(0.0);
    dx.hypot(dy)
}

fn curves(a: &Endpoint, b: &Endpoint) -> Vec<Plan> {
    let d = a.at.dist(b.at);
    if d <= TOLERANCE {
        return Vec::new();
    }
    let mut out = Vec::new();
    for amount in [(d * 0.15).round(), (d * 0.3).round()] {
        if amount < 1.0 {
            continue;
        }
        for left in [true, false] {
            let c = bulge(a.at, b.at, left, amount);
            if square(a, b, c, c) {
                out.push(Plan {
                    hops: vec![Hop::Curve { left, amount }],
                    lane: None,
                });
            }
        }
    }
    if a.side.is_some() || b.side.is_some() {
        let k = (d / 3.0).max(20.0).round();
        out.push(Plan {
            hops: vec![Hop::Via(ctrl(a, k), ctrl(b, k))],
            lane: None,
        });
    }
    out
}

/// The control point of `curve left|right <amount>` from `a` to `b`.
fn bulge(a: Pt, b: Pt, left: bool, amount: f64) -> Pt {
    let d = (b - a).norm();
    let normal = if left {
        Pt::new(d.y, -d.x)
    } else {
        Pt::new(-d.y, d.x)
    };
    a.lerp(b, 0.5) + normal * (2.0 * amount)
}

/// Whether a curve from `a` that leaves toward `c1` and reaches `b` from `c2` leaves and
/// enters side anchors along their normals.
fn square(a: &Endpoint, b: &Endpoint, c1: Pt, c2: Pt) -> bool {
    a.side
        .is_none_or(|s| dot((c1 - a.at).norm(), s.outward()) > SQUARE)
        && b.side
            .is_none_or(|s| dot((b.at - c2).norm(), s.outward()) < -SQUARE)
}

/// A control point `k` px out from an endpoint along its side's normal, written as an offset
/// from the endpoint's reference, folding in the reference's own offset.
fn ctrl(e: &Endpoint, k: f64) -> Ctrl {
    let Some(side) = e.side else {
        return Ctrl {
            at: e.at,
            expr: e.expr.clone(),
        };
    };
    let off = side.outward() * k;
    let expr = e.anchor.as_ref().map(|(base, o)| {
        let d = *o + off;
        format!("{base}+({},{})", num(d.x), num(d.y))
    });
    Ctrl {
        at: e.at + off,
        expr,
    }
}
