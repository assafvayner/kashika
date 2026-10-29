use kashika_geom::Pt;

use crate::Endpoint;
use crate::candidates::{Hop, Plan};

/// A coordinate as a person would write it: at most two decimals, no `-0`.
pub fn num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 { "0".into() } else { format!("{r}") }
}

fn point(at: Pt, expr: &Option<String>, origin: Pt) -> String {
    match expr {
        Some(e) => e.clone(),
        None => format!("({},{})", num(at.x - origin.x), num(at.y - origin.y)),
    }
}

/// The path part of a `line` statement. Literals are written relative to `origin`, the
/// top-left of the group the line belongs to.
pub(crate) fn path_text(from: &Endpoint, to: &Endpoint, plan: &Plan, origin: Pt) -> String {
    let end = point(to.at, &to.expr, origin);
    let mut s = point(from.at, &from.expr, origin);
    for hop in &plan.hops {
        let h = match hop {
            Hop::To => end.clone(),
            Hop::Hv => format!("hv {end}"),
            Hop::Vh => format!("vh {end}"),
            Hop::X(l) => format!(
                "x {}",
                l.expr.clone().unwrap_or_else(|| num(l.value - origin.x))
            ),
            Hop::Y(l) => format!(
                "y {}",
                l.expr.clone().unwrap_or_else(|| num(l.value - origin.y))
            ),
            Hop::Curve { left, amount } => format!(
                "curve {} {} {end}",
                if *left { "left" } else { "right" },
                num(*amount)
            ),
            Hop::Via(c1, c2) => format!(
                "curve via {} {} {end}",
                point(c1.at, &c1.expr, origin),
                point(c2.at, &c2.expr, origin)
            ),
        };
        s.push_str(" -> ");
        s.push_str(&h);
    }
    s
}

/// A `line` statement, or an `arrow` one when `arrow` is set.
pub(crate) fn statement(arrow: bool, id: &str, path: &str, attrs: &str) -> String {
    let kw = if arrow { "arrow" } else { "line" };
    if attrs.is_empty() {
        format!("{kw} {id} {path}")
    } else {
        format!("{kw} {id} {path} {attrs}")
    }
}
