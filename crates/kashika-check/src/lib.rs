pub mod ctx;
pub mod free;
pub mod index;
mod lints;
pub mod outline;
pub mod ports;
mod suppress;
pub mod text_box;

use kashika_diag::Diagnostic;
use kashika_model::Scene;
use serde::Serialize;

pub use suppress::suppress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Metrics {
    pub crossings: usize,
    pub wire_px: u64,
}

/// Runs every warning lint on the scene and marks the suppressed findings.
pub fn check(scene: &Scene) -> Vec<Diagnostic> {
    let ctx = ctx::Ctx::new(scene);
    let mut out = lint(&ctx);
    for d in &mut out {
        free::resolve(&ctx, d);
    }
    out
}

/// Like `check`, but leaves costly help in `Diagnostic::deferred`; `resolve` fills it in for
/// the diagnostics that are shown.
pub fn check_deferred(scene: &Scene) -> Vec<Diagnostic> {
    lint(&ctx::Ctx::new(scene))
}

/// Fills in the help and facts `check_deferred` left out of `diags`.
pub fn resolve<'a>(scene: &Scene, diags: impl IntoIterator<Item = &'a mut Diagnostic>) {
    let mut diags = diags
        .into_iter()
        .filter(|d| !d.deferred.is_empty())
        .peekable();
    if diags.peek().is_none() {
        return;
    }
    let ctx = ctx::Ctx::new(scene);
    for d in diags {
        free::resolve(&ctx, d);
    }
}

fn lint(ctx: &ctx::Ctx) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = lints::ALL.iter().flat_map(|lint| lint(ctx)).collect();
    suppress(ctx.scene, &mut out);
    out
}

/// Summary numbers for the report: crossings (suppressed ones included) and total line length.
pub fn metrics(scene: &Scene, diags: &[Diagnostic]) -> Metrics {
    let wire: f64 = scene
        .lines
        .iter()
        .map(|l| kashika_geom::length(&l.path))
        .sum();
    Metrics {
        crossings: diags.iter().filter(|d| d.code == "cross").count(),
        wire_px: wire.round() as u64,
    }
}
