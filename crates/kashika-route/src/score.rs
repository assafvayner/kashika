//! Scores probe lines with the same checks `kashika check` runs.

use kashika_check::ctx::Ctx;
use kashika_check::free::{Axis, free_intervals};
use kashika_diag::{Diagnostic, code, fact};
use kashika_geom::{Pt, TOLERANCE, length};
use kashika_model::{Line, Scene};

use crate::{Corridor, Crossing, Transit};

pub(crate) struct Score {
    /// Every finding that counts, suppressed ones included.
    pub(crate) findings: Vec<Diagnostic>,
    pub(crate) crossed: Vec<Crossing>,
    pub(crate) transited: Vec<Transit>,
    pub(crate) near: usize,
    pub(crate) length: f64,
}

/// Scores `probe` with the findings `kashika check` reports for it, suppressed ones included.
/// Findings that also name `ignore`, the line being rerouted, don't count. `None` means the
/// probe enters a box.
pub(crate) fn score(scene: &Scene, probe: &Line, ignore: Option<&str>) -> Option<Score> {
    let mine: Vec<Diagnostic> = kashika_check::check_deferred(scene)
        .into_iter()
        .filter(|d| d.code != code::UNUSED_ALLOW)
        .filter(|d| involves(d, &probe.id) && !ignore.is_some_and(|id| involves(d, id)))
        .collect();
    if mine.iter().any(|d| d.code == code::THROUGH) {
        return None;
    }
    let crossed = mine
        .iter()
        .filter(|d| d.code == code::CROSS)
        .filter_map(|d| {
            Some(Crossing {
                line: d
                    .elements
                    .iter()
                    .map(|e| e.id.as_str())
                    .find(|id| *id != probe.id)
                    .unwrap_or_default()
                    .to_string(),
                at: at(d, fact::AT)?,
            })
        })
        .collect();
    let transited = mine
        .iter()
        .filter(|d| d.code == code::GROUP_TRANSIT)
        .filter_map(|d| {
            let group = d.elements.iter().find(|e| e.kind == "group")?;
            Some(Transit {
                group: group.id.clone(),
                at: at(d, fact::ENTRY)?,
            })
        })
        .collect();
    let near = mine
        .iter()
        .filter(|d| d.code == code::NEAR || d.code == code::SHARED_SEGMENT)
        .count();
    Some(Score {
        findings: mine,
        crossed,
        transited,
        near,
        length: length(&probe.path),
    })
}

/// Whether the `allow=` entry `entry` (`code` or `code:other`) on the line `owner` suppresses
/// `d`, as `kashika check` decides.
pub(crate) fn suppresses(entry: &str, owner: &str, d: &Diagnostic) -> bool {
    let (code, other) = match entry.split_once(':') {
        Some((c, o)) => (c, Some(o)),
        None => (entry, None),
    };
    kashika_check::suppresses(code, other, owner, d)
}

fn involves(d: &Diagnostic, id: &str) -> bool {
    d.elements.iter().any(|e| e.id == id)
}

/// The point `fact` of `d`, which its lint always writes.
fn at(d: &Diagnostic, fact: &str) -> Option<Pt> {
    let p = d.point(fact);
    debug_assert!(p.is_some(), "`{}` has no `{fact}` fact", d.code);
    p.map(|[x, y]| Pt::new(x, y))
}

/// The free interval, as `kashika inspect --corridors` reports it, that holds a lane at
/// `value` running across `span`. Lines in `skip` aren't obstacles. `None` when the lane
/// doesn't keep `clearance`.
pub(crate) fn corridor(
    ctx: &Ctx,
    axis: Axis,
    value: f64,
    span: (f64, f64),
    skip: &[&str],
) -> Option<Corridor> {
    let within = match axis {
        Axis::X => (0.0, ctx.scene.size.0),
        Axis::Y => (0.0, ctx.scene.size.1),
    };
    free_intervals(ctx, axis, span, within, skip)
        .into_iter()
        .find(|&(lo, hi)| lo - TOLERANCE <= value && value <= hi + TOLERANCE)
        .map(|(lo, hi)| Corridor {
            axis,
            band: [lo, hi],
        })
}
