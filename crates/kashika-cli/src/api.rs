//! The CLI's commands as plain functions over a source string, for `kashika-wasm`: no
//! files, no process exit, no stdout. Text reports are the CLI's, with the input named
//! `NAME`.

use kashika_check::Deferred;
use kashika_check::ctx::Ctx;
use kashika_diag::Diagnostic;
use serde::Serialize;

use crate::report;

/// The input name the text reports show.
pub const NAME: &str = "diagram.kash";

#[derive(Debug, Serialize)]
pub struct Check {
    pub errors: usize,
    pub warnings: usize,
    /// Every diagnostic, suppressed ones included, in report order.
    pub diagnostics: Vec<Diagnostic>,
    /// `kashika check` text output.
    pub text: String,
}

fn opts() -> kashika_model::Options {
    kashika_model::Options::default()
}

/// `kashika check` on `src`.
pub fn check(src: &str) -> Check {
    let kashika_model::Compiled { scene, diags, .. } = kashika_model::compile_file(src, &opts());
    let Some(scene) = scene else {
        let mut diags = diags;
        let text = report::text(NAME, src, &mut diags, &report::Opts::bare(), None);
        let counts = report::counts(&diags, &[]);
        return Check {
            errors: counts.errors,
            warnings: counts.warns,
            diagnostics: diags,
            text,
        };
    };
    let ctx = Ctx::new(&scene);
    let found = kashika_check::check_bounded(&ctx, report::KEEP, &|_| true);
    let metrics = found.metrics(&scene);
    let mut all: Vec<(Diagnostic, Vec<Deferred>)> = diags
        .into_iter()
        .map(|d| (d, Vec::new()))
        .chain(found.diags.into_iter().zip(found.deferred))
        .collect();
    all.sort_by(|a, b| kashika_diag::order(&a.0, &b.0));
    let (mut diags, deferred): (Vec<_>, Vec<_>) = all.into_iter().unzip();
    let omitted = found.omitted;
    let report_opts = report::Opts {
        max: 50,
        show_suppressed: false,
        summary: false,
        metrics: Some(metrics),
        deferred: Some((&ctx, &deferred)),
        omitted: &omitted,
    };
    let text = report::text(NAME, src, &mut diags, &report_opts, None);
    let counts = report::counts(&diags, &omitted);
    Check {
        errors: counts.errors,
        warnings: counts.warns,
        diagnostics: diags,
        text,
    }
}

/// `kashika fmt`: the formatted source, or the text report of the parse errors.
pub fn format(src: &str) -> Result<String, String> {
    kashika_syntax::format(src)
        .map_err(|mut diags| report::text(NAME, src, &mut diags, &report::Opts::bare(), None))
}
