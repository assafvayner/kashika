pub mod ctx;
pub mod element;
pub mod free;
pub mod index;
mod lints;
pub mod outline;
pub mod ports;
mod sink;
mod suppress;
pub mod text_box;

use kashika_diag::{Diagnostic, code};
use kashika_model::Scene;
use serde::Serialize;

pub use free::{Deferred, DeferredKind, Finding};
pub use sink::{Bounded, Omitted};
pub use suppress::suppress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Metrics {
    pub crossings: usize,
    pub wire_px: u64,
}

/// Runs every warning lint on the scene and marks the suppressed findings.
pub fn check(scene: &Scene) -> Vec<Diagnostic> {
    let ctx = ctx::Ctx::new(scene);
    let found = lint(&ctx, sink::Sink::new(scene, usize::MAX, None));
    found
        .diags
        .into_iter()
        .zip(found.deferred)
        .map(|(mut d, deferred)| {
            free::resolve(&ctx, &mut d, deferred);
            d
        })
        .collect()
}

/// Like `check`, but without the costly help that `check_bounded` defers.
pub fn check_deferred(scene: &Scene) -> Vec<Diagnostic> {
    lint(
        &ctx::Ctx::new(scene),
        sink::Sink::new(scene, usize::MAX, None),
    )
    .diags
}

/// Like `check`, with memory bounded for scenes with huge numbers of findings and the costly
/// help left in `Bounded::deferred`, for `free::resolve` to work out for the diagnostics that
/// are shown. Only findings that pass `filter` are reported. Of those with the same code and
/// suppression, the first `keep` in `kashika_diag::sort` order are kept and the rest only
/// counted in `omitted`. Kept findings come in the order `check_deferred` reports them.
pub fn check_bounded(ctx: &ctx::Ctx, keep: usize, filter: &dyn Fn(&Diagnostic) -> bool) -> Bounded {
    lint(ctx, sink::Sink::new(ctx.scene, keep, Some(filter)))
}

fn lint(ctx: &ctx::Ctx, mut sink: sink::Sink) -> Bounded {
    for lint in lints::ALL {
        lint(ctx, &mut sink);
    }
    sink.finish()
}

/// Summary numbers for the report: crossings (suppressed ones included) and total line length.
pub fn metrics(scene: &Scene, diags: &[Diagnostic]) -> Metrics {
    let crossings = diags.iter().filter(|d| d.code == code::CROSS).count();
    metrics_with(scene, crossings)
}

fn metrics_with(scene: &Scene, crossings: usize) -> Metrics {
    let wire: f64 = scene
        .lines
        .iter()
        .map(|l| kashika_geom::length(&l.path))
        .sum();
    Metrics {
        crossings,
        wire_px: wire.round() as u64,
    }
}

impl Bounded {
    /// Like `metrics`, counting every crossing found, also those filtered out or omitted.
    pub fn metrics(&self, scene: &Scene) -> Metrics {
        metrics_with(scene, self.found(code::CROSS))
    }
}
