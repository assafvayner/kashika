//! Scores probe lines with the same checks `kashika check` runs.

use kashika_check::ctx::Ctx;
use kashika_check::free::{Axis, free_intervals};
use kashika_diag::Diagnostic;
use kashika_geom::{Pt, TOLERANCE, length};
use kashika_model::{Line, Scene};

use crate::{Corridor, Crossing};

pub struct Score {
    pub crossed: Vec<Crossing>,
    pub near: usize,
    pub length: f64,
}

/// Scores `probe` with the findings `kashika check` reports for it, suppressed ones included.
/// Findings that also name `ignore`, the line being rerouted, don't count. `None` means the
/// probe enters a box.
pub fn score(scene: &Scene, probe: &Line, ignore: Option<&str>) -> Option<Score> {
    let mine: Vec<Diagnostic> = kashika_check::check_deferred(scene)
        .into_iter()
        .filter(|d| involves(d, &probe.id) && !ignore.is_some_and(|id| involves(d, id)))
        .collect();
    if mine.iter().any(|d| d.code == "through") {
        return None;
    }
    let crossed = mine
        .iter()
        .filter(|d| d.code == "cross")
        .map(|d| Crossing {
            line: d
                .elements
                .iter()
                .map(|e| e.id.as_str())
                .find(|id| *id != probe.id)
                .unwrap_or_default()
                .to_string(),
            at: at(d),
        })
        .collect();
    let near = mine
        .iter()
        .filter(|d| d.code == "near" || d.code == "shared-segment")
        .count();
    Some(Score {
        crossed,
        near,
        length: length(&probe.path),
    })
}

fn involves(d: &Diagnostic, id: &str) -> bool {
    d.elements.iter().any(|e| e.id == id)
}

fn at(d: &Diagnostic) -> Pt {
    let c = |i: usize| d.facts.get("at").and_then(|v| v[i].as_f64());
    match (c(0), c(1)) {
        (Some(x), Some(y)) => Pt::new(x, y),
        _ => Pt::default(),
    }
}

/// The free interval, as `kashika inspect --corridors` reports it, that holds a lane at
/// `value` running across `span`. Lines in `skip` aren't obstacles. `None` when the lane
/// doesn't keep `clearance`.
pub fn corridor(
    scene: &Scene,
    axis: &'static str,
    value: f64,
    span: (f64, f64),
    skip: &[&str],
) -> Option<Corridor> {
    let (free_axis, within) = if axis == "x" {
        (Axis::X, (0.0, scene.size.0))
    } else {
        (Axis::Y, (0.0, scene.size.1))
    };
    free_intervals(&Ctx::new(scene), free_axis, span, within, skip)
        .into_iter()
        .find(|&(lo, hi)| lo - TOLERANCE <= value && value <= hi + TOLERANCE)
        .map(|(lo, hi)| Corridor {
            axis,
            band: [lo, hi],
        })
}
