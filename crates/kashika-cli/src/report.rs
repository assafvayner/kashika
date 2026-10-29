use std::collections::BTreeMap;

use kashika_check::{Metrics, Omitted};
use kashika_diag::{Diagnostic, Severity};
use kashika_model::Scene;
use sha2::{Digest, Sha256};

use crate::term;

pub const SCHEMA: &str = "kashika.check/1";

pub struct Output {
    pub path: String,
    pub written: bool,
}

pub struct Opts<'a> {
    pub max: usize,
    pub show_suppressed: bool,
    pub summary: bool,
    pub metrics: Option<Metrics>,
    /// The scene the diagnostics' deferred help is resolved against.
    pub scene: Option<&'a Scene>,
    /// Warnings counted but not kept, past `KEEP` (or `max`) per code.
    pub omitted: &'a [Omitted],
}

/// How many warnings per code `check` keeps at least, so that memory stays bounded; the
/// rest are only counted.
pub const KEEP: usize = 1000;

pub struct Counts {
    pub errors: usize,
    pub warns: usize,
    pub suppressed: usize,
}

pub fn counts(diags: &[Diagnostic], omitted: &[Omitted]) -> Counts {
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let suppressed = diags.iter().filter(|d| d.suppressed).count();
    let more = |s: bool| -> usize {
        omitted
            .iter()
            .filter(|o| o.suppressed == s)
            .map(|o| o.count)
            .sum()
    };
    Counts {
        errors,
        warns: diags.len() - errors - suppressed + more(false),
        suppressed: suppressed + more(true),
    }
}

/// Omitted warnings that would be visible.
fn omitted_visible(opts: &Opts) -> usize {
    opts.omitted
        .iter()
        .filter(|o| opts.show_suppressed || !o.suppressed)
        .map(|o| o.count)
        .sum()
}

/// Per code: how many findings are active and how many are suppressed.
pub fn by_code(
    diags: &[Diagnostic],
    omitted: &[Omitted],
) -> BTreeMap<&'static str, (usize, usize)> {
    let mut m: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for d in diags {
        let e = m.entry(d.code).or_default();
        if d.suppressed {
            e.1 += 1;
        } else {
            e.0 += 1;
        }
    }
    for o in omitted {
        let e = m.entry(o.code).or_default();
        if o.suppressed {
            e.1 += o.count;
        } else {
            e.0 += o.count;
        }
    }
    m
}

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn visible(diags: &[Diagnostic], show_suppressed: bool) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| show_suppressed || !d.suppressed)
        .collect()
}

/// What the report does with a visible diagnostic.
#[derive(Clone, Copy, PartialEq)]
enum Show {
    Print,
    /// Cut by `--max`.
    Over,
}

/// Every error is printed; warnings fill what is left of `max`, in source order.
fn plan(shown: &[&Diagnostic], max: usize) -> Vec<Show> {
    let errors = shown
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let mut budget = max.saturating_sub(errors);
    shown
        .iter()
        .map(|d| {
            if d.severity != Severity::Error {
                if budget == 0 {
                    return Show::Over;
                }
                budget -= 1;
            }
            Show::Print
        })
        .collect()
}

struct Selection {
    /// Indices into the diagnostics of the visible ones.
    idx: Vec<usize>,
    plan: Vec<Show>,
}

/// Picks what to show, errors first and each severity in source order, and fills in the
/// deferred help of the diagnostics that are printed.
fn select(diags: &mut [Diagnostic], opts: &Opts) -> Selection {
    let mut idx: Vec<usize> = (0..diags.len())
        .filter(|&i| opts.show_suppressed || !diags[i].suppressed)
        .collect();
    idx.sort_by_key(|&i| diags[i].severity != Severity::Error);
    let shown: Vec<&Diagnostic> = idx.iter().map(|&i| &diags[i]).collect();
    let plan = plan(&shown, opts.max);
    if let Some(scene) = opts.scene {
        let mut print = vec![false; diags.len()];
        for (&i, &s) in idx.iter().zip(&plan) {
            print[i] = s == Show::Print;
        }
        kashika_check::resolve(
            scene,
            diags
                .iter_mut()
                .zip(print)
                .filter(|(_, p)| *p)
                .map(|(d, _)| d),
        );
    }
    Selection { idx, plan }
}

pub fn text(
    name: &str,
    src: &str,
    diags: &mut [Diagnostic],
    opts: &Opts,
    output: Option<&Output>,
) -> String {
    let sel = select(diags, opts);
    let diags = &*diags;
    let name = term::line(name);
    let lines: Vec<&str> = src.lines().collect();
    let source = |out: &mut String, line: u32| {
        if let Some(l) = (line as usize).checked_sub(1).and_then(|i| lines.get(i)) {
            out.push_str(&format!("{line:>5} | {}\n", term::line(l)));
        }
    };
    let mut overflow = omitted_visible(opts);
    let mut out = String::new();
    for (&i, show) in sel.idx.iter().zip(&sel.plan) {
        if *show == Show::Over {
            overflow += 1;
            continue;
        }
        let d = &diags[i];
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warn => "warn",
        };
        let note = if d.suppressed { " (suppressed)" } else { "" };
        out.push_str(&format!(
            "{name}:{}:{}: {sev}[{}]: {}{note}\n",
            d.span.line,
            d.span.col,
            d.code,
            term::line(&d.message)
        ));
        let mut seen = vec![d.span.line];
        source(&mut out, d.span.line);
        for e in &d.elements {
            if !seen.contains(&e.span.line) {
                seen.push(e.span.line);
                source(&mut out, e.span.line);
            }
        }
        for h in &d.help {
            out.push_str(&format!("      = help: {}\n", term::line(h)));
        }
        if let Some(r) = &d.reason {
            out.push_str(&format!("      = reason: {}\n", term::line(r)));
        }
    }
    if overflow > 0 {
        out.push_str(&format!("... and {overflow} more\n"));
    }
    if opts.summary {
        out.push_str("by code:\n");
        for (code, (active, suppressed)) in by_code(diags, opts.omitted) {
            out.push_str(&format!("  {code}: {active}"));
            if suppressed > 0 {
                out.push_str(&format!(" ({suppressed} suppressed)"));
            }
            out.push('\n');
        }
    }
    let c = counts(diags, opts.omitted);
    out.push_str(&format!(
        "{}, {}",
        plural(c.errors, "error"),
        plural(c.warns, "warning")
    ));
    if c.suppressed > 0 {
        out.push_str(&format!(" ({} suppressed)", c.suppressed));
    }
    if let Some(m) = opts.metrics {
        out.push_str(&format!(
            " · crossings={} wire={}px",
            m.crossings, m.wire_px
        ));
    }
    if let Some(o) = output {
        out.push_str(if o.written {
            " · output: written"
        } else {
            " · output: not written"
        });
    }
    out.push('\n');
    out
}

pub fn json(
    name: &str,
    src: &str,
    diags: &mut [Diagnostic],
    opts: &Opts,
    output: Option<&Output>,
) -> String {
    let sel = select(diags, opts);
    let listed: Vec<&Diagnostic> = sel
        .idx
        .iter()
        .zip(&sel.plan)
        .filter(|(_, s)| **s == Show::Print)
        .map(|(&i, _)| &diags[i])
        .collect();
    let c = counts(diags, opts.omitted);
    let v = serde_json::json!({
        "schema": SCHEMA,
        "tool": env!("CARGO_PKG_VERSION"),
        "input": { "name": name, "sha256": format!("{:x}", Sha256::digest(src.as_bytes())) },
        "output": output.map(|o| serde_json::json!({ "path": o.path, "written": o.written })),
        "counts": { "error": c.errors, "warn": c.warns, "suppressed": c.suppressed },
        "by_code": by_code(diags, opts.omitted)
            .into_iter()
            .map(|(k, (active, suppressed))| (k, serde_json::json!({ "active": active, "suppressed": suppressed })))
            .collect::<BTreeMap<_, _>>(),
        "truncated": listed.len() < sel.idx.len() || omitted_visible(opts) > 0,
        "metrics": opts.metrics,
        "diagnostics": listed,
    });
    format!("{v}\n")
}
