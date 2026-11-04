//! The CLI's commands as plain functions over a source string, for `kashika-wasm`: no
//! files, no process exit, no stdout. Text reports are the CLI's, with the input named
//! `NAME`.

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
    /// The diagnostics the report kept, suppressed ones included, in report order, with
    /// their help resolved. Warnings past the per-code cap are only counted in `warnings`.
    pub diagnostics: Vec<Diagnostic>,
    /// `kashika check` text output.
    pub text: String,
}

/// `kashika check` on `src`.
pub fn check(src: &str) -> Check {
    let kashika_model::Compiled { scene, diags, .. } =
        kashika_model::compile_file(src, &kashika_model::Options::default());
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
    let report::Linted {
        mut diags,
        deferred,
        omitted,
        metrics,
    } = report::lint(&scene, &ctx, diags, report::KEEP, &|_| true);
    for (d, deferred) in diags.iter_mut().zip(deferred) {
        kashika_check::free::resolve(&ctx, d, deferred);
    }
    let opts = report::Opts {
        metrics: Some(metrics),
        omitted: &omitted,
        ..report::Opts::bare()
    };
    let text = report::text(NAME, src, &mut diags, &opts, None);
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
