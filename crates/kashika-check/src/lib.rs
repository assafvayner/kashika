pub mod ctx;
pub mod free;
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
    let mut out: Vec<Diagnostic> = lints::ALL.iter().flat_map(|lint| lint(&ctx)).collect();
    suppress(scene, &mut out);
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
