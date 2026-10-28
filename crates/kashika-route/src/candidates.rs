use kashika_geom::Pt;
use kashika_model::{Scene, ShapeKind};

use crate::text::num;
use crate::{Endpoint, Style};

/// An `x` or `y` target: relative to a box corner where possible, otherwise a literal.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    pub value: f64,
    pub expr: Option<String>,
}

/// A `curve via` control point.
#[derive(Debug, Clone, PartialEq)]
pub struct Ctrl {
    pub at: Pt,
    pub expr: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Hop {
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
pub struct Plan {
    pub hops: Vec<Hop>,
    /// The lane of a two-elbow path: `("x", v)` when its middle segment is vertical at x=v.
    pub lane: Option<(&'static str, f64)>,
}

pub fn generate(scene: &Scene, from: &Endpoint, to: &Endpoint, style: Style) -> Vec<Plan> {
    match style {
        Style::Bezier => curves(from, to),
        Style::Corner | Style::Arc => orthogonal(scene, from, to),
    }
}

fn orthogonal(scene: &Scene, a: &Endpoint, b: &Endpoint) -> Vec<Plan> {
    let simple = |hops: Vec<Hop>| Plan { hops, lane: None };
    let mut out = Vec::new();
    if (a.at.x - b.at.x).abs() <= 0.5 || (a.at.y - b.at.y).abs() <= 0.5 {
        out.push(simple(vec![Hop::To]));
    }
    out.push(simple(vec![Hop::Hv]));
    out.push(simple(vec![Hop::Vh]));
    for lane in lanes(scene, a.at.x, b.at.x, "x") {
        let v = lane.value;
        out.push(Plan {
            hops: vec![Hop::X(lane), Hop::Vh],
            lane: Some(("x", v)),
        });
    }
    for lane in lanes(scene, a.at.y, b.at.y, "y") {
        let v = lane.value;
        out.push(Plan {
            hops: vec![Hop::Y(lane), Hop::Hv],
            lane: Some(("y", v)),
        });
    }
    out
}

/// Lane positions on one axis: the midpoint between the endpoints, and `2 × clearance`
/// outside each box edge, written relative to the box's corner so the lane follows the box.
fn lanes(scene: &Scene, a: f64, b: f64, axis: &str) -> Vec<Lane> {
    let g = 2.0 * scene.clearance;
    let gap = num(g);
    let limit = if axis == "x" {
        scene.size.0
    } else {
        scene.size.1
    };
    let mid = match scene.grid {
        Some(grid) if grid > 0.0 => ((a + b) / 2.0 / grid).round() * grid,
        _ => ((a + b) / 2.0).round(),
    };
    let mut out = vec![Lane {
        value: mid,
        expr: None,
    }];
    for s in scene.shapes.iter().filter(|s| s.kind != ShapeKind::Group) {
        let (r, id) = (s.rect, &s.id);
        if axis == "x" {
            out.push(Lane {
                value: r.x - g,
                expr: Some(format!("{id}.tl+(-{gap},0)")),
            });
            out.push(Lane {
                value: r.right() + g,
                expr: Some(format!("{id}.tr+({gap},0)")),
            });
        } else {
            out.push(Lane {
                value: r.y - g,
                expr: Some(format!("{id}.tl+(0,-{gap})")),
            });
            out.push(Lane {
                value: r.bottom() + g,
                expr: Some(format!("{id}.bl+(0,{gap})")),
            });
        }
    }
    out.retain(|l| l.value > 0.0 && l.value < limit);
    out
}

fn curves(a: &Endpoint, b: &Endpoint) -> Vec<Plan> {
    let d = a.at.dist(b.at);
    let mut out = Vec::new();
    for amount in [(d * 0.15).round(), (d * 0.3).round()] {
        if amount >= 1.0 {
            for left in [true, false] {
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

/// A control point `k` px out from an endpoint along its side's normal, written as an offset
/// from the endpoint's reference when that reference has no offset of its own.
fn ctrl(e: &Endpoint, k: f64) -> Ctrl {
    let Some(side) = e.side else {
        return Ctrl {
            at: e.at,
            expr: e.expr.clone(),
        };
    };
    let off = side.outward() * k;
    let expr = e
        .expr
        .as_ref()
        .filter(|x| !x.contains('+'))
        .map(|x| format!("{x}+({},{})", num(off.x), num(off.y)));
    Ctrl {
        at: e.at + off,
        expr,
    }
}
