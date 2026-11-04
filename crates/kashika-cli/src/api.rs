//! The CLI's commands as plain functions over a source string, for `kashika-wasm`: no
//! files, no process exit, no stdout. Text reports are the CLI's, with the input named
//! `NAME`.

use kashika_check::ctx::Ctx;
use kashika_diag::Diagnostic;
use kashika_geom::round2;
use kashika_model::Scene;
use kashika_route::{Error, Request, Style, Suggestion};
use serde::Serialize;

use crate::{icons, inspect, measure, ref_cmd, report, route_cmd};

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

/// `kashika inspect`: the text report and the JSON report.
pub fn inspect(
    src: &str,
    ids: &[String],
    ports: bool,
) -> Result<(String, inspect::Inspect), String> {
    let data = crate::inspect_data(
        NAME,
        src,
        &kashika_model::Options::default(),
        ids,
        ports,
        None,
    )?;
    Ok((crate::term::text(&inspect::text(&data)).into_owned(), data))
}

#[derive(Debug)]
pub enum RouteError {
    /// Bad arguments.
    Usage(String),
    /// The input has errors: the `check` text report.
    Input(String),
    /// `from` or `to` doesn't resolve: one `error[...]` line per diagnostic, with help.
    Endpoints(String),
}

/// `kashika route`: the text report and the suggestion. `style` is `corner`, `arc` or
/// `bezier`; `line` reroutes an existing line.
pub fn route(
    src: &str,
    from: Option<&str>,
    to: Option<&str>,
    style: &str,
    line: Option<&str>,
    radius: f64,
    max: usize,
) -> Result<(String, Suggestion), RouteError> {
    let style = match style {
        "corner" => Style::Corner,
        "arc" => Style::Arc,
        "bezier" => Style::Bezier,
        other => {
            return Err(RouteError::Usage(format!(
                "unknown style `{other}`; use corner, arc or bezier"
            )));
        }
    };
    if !(radius.is_finite() && radius >= 0.0) {
        return Err(RouteError::Usage(
            "radius must be a finite number, 0 or more".into(),
        ));
    }
    if max == 0 {
        return Err(RouteError::Usage("max must be 1 or more".into()));
    }
    let opts = kashika_model::Options::default();
    let req = Request {
        src,
        from,
        to,
        style,
        line,
        radius,
        max,
        opts: &opts,
    };
    match kashika_route::suggest(&req) {
        Ok(s) => Ok((crate::term::text(&route_cmd::text(&s)).into_owned(), s)),
        Err(Error::Usage(msg)) => Err(RouteError::Usage(msg)),
        Err(Error::Input(mut diags)) => Err(RouteError::Input(report::text(
            NAME,
            src,
            &mut diags,
            &report::Opts::bare(),
            None,
        ))),
        Err(Error::Endpoints(diags)) => Err(RouteError::Endpoints(route_cmd::endpoints_text(
            NAME, &diags,
        ))),
    }
}

#[derive(Debug, Serialize)]
pub struct Measured {
    pub width: f64,
    pub height: f64,
    pub lines: usize,
}

/// `kashika measure` of one text, with `\n` escapes as in a `.kash` string.
pub fn measure(text: &str, size: f64, bold: bool) -> Result<Measured, String> {
    if !(size.is_finite() && size > 0.0) {
        return Err("size must be a positive number".into());
    }
    let block = kashika_text::measure(&measure::unescape(text), size, bold)
        .map_err(|e| format!("error[parse]: {e}"))?;
    Ok(Measured {
        width: round2(block.width()),
        height: round2(block.height()),
        lines: block.lines().len(),
    })
}

/// `kashika ref [topic]`.
pub fn ref_page(topic: Option<&str>) -> Result<String, String> {
    ref_cmd::page(topic)
}

/// `kashika icons [--search term]` over the bundled icons and built-in shapes.
pub fn icons(search: Option<&str>) -> Vec<icons::Entry> {
    icons::list(&[], search)
}

/// A shape or text with its canvas box, for hit testing in a viewer.
#[derive(Debug, Serialize)]
pub struct Elem {
    pub id: String,
    /// `box`, `group`, `image` or `text`.
    pub kind: &'static str,
    pub parent: Option<String>,
    /// Nesting depth: 0 at the top level, so the deepest hit is the innermost element.
    pub depth: usize,
    pub bbox: [[f64; 2]; 2],
}

fn bbox(r: kashika_geom::Rect) -> [[f64; 2]; 2] {
    [
        [round2(r.x), round2(r.y)],
        [round2(r.x + r.w), round2(r.y + r.h)],
    ]
}

/// Every shape and text of `src`, however broken, with its box.
pub fn elements(src: &str) -> Vec<Elem> {
    let (scene, _) = kashika_model::compile_with(src, &kashika_model::Options::default());
    elements_of(&scene)
}

fn elements_of(scene: &Scene) -> Vec<Elem> {
    let depth_of = |parent: &Option<String>| {
        parent
            .as_ref()
            .and_then(|p| scene.shapes.iter().find(|s| &s.id == p))
            .map_or(0, |s| s.depth + 1)
    };
    scene
        .shapes
        .iter()
        .map(|s| Elem {
            id: s.id.clone(),
            kind: s.kind.name(),
            parent: s.parent.clone(),
            depth: s.depth,
            bbox: bbox(s.rect),
        })
        .chain(scene.texts.iter().map(|t| Elem {
            id: t.display_id(),
            kind: "text",
            parent: t.parent.clone(),
            depth: depth_of(&t.parent),
            bbox: bbox(t.rect),
        }))
        .collect()
}
