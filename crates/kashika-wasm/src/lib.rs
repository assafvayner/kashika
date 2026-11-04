//! Checks and renders Kashika source in WebAssembly, for editors, Markdown previews and the
//! web app: `render`, plus the CLI's `check`, `inspect`, `route`, `measure`, `ref`, `icons`
//! and `fmt` as JSON over a source string. There is no file system: imports, images and
//! `icons=` directories report not found, while built-in shapes and bundled icons work.

use kashika::api;
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
    /// Every shape and text with its box; empty when the source is a library.
    pub elements: Vec<api::Elem>,
}

/// Compiles, checks and renders `src`. With `force`, a diagram with errors still renders,
/// with error markers like `kashika render --force`.
pub fn render(src: &str, force: bool) -> Rendered {
    let compiled = kashika_model::compile_file(src, &kashika_model::Options::default());
    let mut diags = compiled.diags;
    let elements = compiled
        .scene
        .as_ref()
        .map(api::elements_of)
        .unwrap_or_default();
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
    kashika_diag::sort(&mut diags);
    Rendered {
        svg,
        errors: count(&diags, Severity::Error),
        warnings: count(&diags, Severity::Warn),
        diagnostics: diags,
        elements,
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
    json(&render(src, force))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

/// Reports a panic's message and location on the JS console before wasm aborts, instead
/// of an opaque `unreachable` trap.
#[wasm_bindgen(start)]
pub fn init() {
    std::panic::set_hook(Box::new(|info| {
        console_error(&format!("kashika panicked: {info}"))
    }));
}

/// The Kashika version these bindings were built from.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn json<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("a report serializes to JSON")
}

fn error(msg: &str) -> String {
    json(&serde_json::json!({ "error": msg }))
}

/// `kashika check` as JSON: `{errors, warnings, diagnostics, text}`.
#[wasm_bindgen]
pub fn check(src: &str) -> String {
    json(&api::check(src))
}

/// `kashika inspect` as JSON: `{text, report}`, or `{error}` for an unknown id.
#[wasm_bindgen]
pub fn inspect(src: &str, ids: Vec<String>, ports: bool) -> String {
    match api::inspect(src, &ids, ports) {
        Ok((text, report)) => json(&serde_json::json!({ "text": text, "report": report })),
        Err(e) => error(&e),
    }
}

/// `kashika route` as JSON: `{text, suggestion}`, or `{error, kind}` with `kind` one of
/// `usage`, `input` (the check report) or `endpoints`.
#[wasm_bindgen]
pub fn route(
    src: &str,
    from: Option<String>,
    to: Option<String>,
    style: &str,
    line: Option<String>,
    radius: f64,
    max: usize,
) -> String {
    match api::route(
        src,
        from.as_deref(),
        to.as_deref(),
        style,
        line.as_deref(),
        radius,
        max,
    ) {
        Ok((text, s)) => json(&serde_json::json!({ "text": text, "suggestion": s })),
        Err(e) => {
            let (kind, msg) = match e {
                api::RouteError::Usage(m) => ("usage", m),
                api::RouteError::Input(m) => ("input", m),
                api::RouteError::Endpoints(m) => ("endpoints", m),
            };
            json(&serde_json::json!({ "error": msg, "kind": kind }))
        }
    }
}

/// `kashika measure` as JSON: `{width, height, lines}` or `{error}`.
#[wasm_bindgen]
pub fn measure(text: &str, size: f64, bold: bool) -> String {
    match api::measure(text, size, bold) {
        Ok(m) => json(&m),
        Err(e) => error(&e),
    }
}

/// `kashika ref [topic]` as JSON: `{page}` or `{error}`.
#[wasm_bindgen(js_name = ref)]
pub fn ref_page(topic: Option<String>) -> String {
    match api::ref_page(topic.as_deref()) {
        Ok(page) => json(&serde_json::json!({ "page": page })),
        Err(e) => error(&e),
    }
}

/// `kashika icons [--search term]` as JSON: `{icons: [{name, path, size, library, matched_keyword}]}`.
#[wasm_bindgen]
pub fn icons(search: Option<String>) -> String {
    json(&serde_json::json!({ "icons": api::icons(search.as_deref()) }))
}

/// `kashika fmt` as JSON: `{source}` or `{error}` (the parse report).
#[wasm_bindgen]
pub fn format(src: &str) -> String {
    match api::format(src) {
        Ok(source) => json(&serde_json::json!({ "source": source })),
        Err(e) => error(&e),
    }
}

/// Every shape and text with its box, as JSON `{elements: [{id, kind, parent, depth, bbox}]}`.
#[wasm_bindgen]
pub fn elements(src: &str) -> String {
    json(&serde_json::json!({ "elements": api::elements(src) }))
}
