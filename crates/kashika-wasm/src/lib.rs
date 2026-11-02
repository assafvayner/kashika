//! Checks and renders Kashika source to SVG in WebAssembly, for editors and Markdown
//! previews. There is no file system: imports, images and `icons=` directories report
//! not found, while built-in shapes and bundled icons work.

use kashika_diag::{Diagnostic, Severity};
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Serialize)]
pub struct Rendered {
    /// The SVG, or `None` when there are errors and `force` is off, or the source is a
    /// library.
    pub svg: Option<String>,
    pub errors: usize,
    pub warnings: usize,
    /// Every diagnostic, suppressed ones included, in report order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Compiles, checks and renders `src`. With `force`, a diagram with errors still renders,
/// with error markers like `kashika render --force`.
pub fn render(src: &str, force: bool) -> Rendered {
    let compiled = kashika_model::compile_file(src, &kashika_model::Options::default());
    let mut diags = compiled.diags;
    let svg = compiled.scene.and_then(|scene| {
        diags.extend(kashika_check::check(&scene));
        kashika_diag::sort(&mut diags);
        let errors = count(&diags, Severity::Error);
        (errors == 0 || force).then(|| {
            let overlay = kashika_render::Overlay {
                error_markers: errors > 0,
                debug: false,
            };
            kashika_render::render_svg_with(&scene, diags.iter().filter(|d| !d.suppressed), overlay)
        })
    });
    Rendered {
        svg,
        errors: count(&diags, Severity::Error),
        warnings: count(&diags, Severity::Warn),
        diagnostics: diags,
    }
}

fn count(diags: &[Diagnostic], severity: Severity) -> usize {
    diags
        .iter()
        .filter(|d| d.severity == severity && !d.suppressed)
        .count()
}

/// `render` as a JSON string: `{svg, errors, warnings, diagnostics}`.
#[wasm_bindgen(js_name = render)]
pub fn render_json(src: &str, force: bool) -> String {
    serde_json::to_string(&render(src, force)).expect("a report serializes to JSON")
}

/// The Kashika version these bindings were built from.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
