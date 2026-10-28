use std::collections::BTreeMap;

use kashika_check::Metrics;
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
    pub metrics: Option<Metrics>,
    /// The scene the diagnostics' deferred help is resolved against.
    pub scene: Option<&'a Scene>,
}

pub struct Counts {
    pub errors: usize,
    pub warns: usize,
    pub suppressed: usize,
}

pub fn counts(diags: &[Diagnostic]) -> Counts {
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let suppressed = diags.iter().filter(|d| d.suppressed).count();
    Counts {
        errors,
        warns: diags.len() - errors - suppressed,
        suppressed,
    }
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

/// How many diagnostics with the same severity, code and message template the text output
/// prints before collapsing the rest into one `... and N more like this` line.
const ALIKE_SHOWN: usize = 3;

type Key = (Severity, &'static str, bool, String);

/// The message with quoted text and numbers blanked, so diagnostics that differ only in
/// the names or values they mention share a key.
fn key(d: &Diagnostic) -> Key {
    let mut template = String::new();
    let mut word = String::new();
    let mut quoted = false;
    let flush = |template: &mut String, word: &mut String| {
        if !word.chars().all(|c| c.is_ascii_digit()) {
            template.push_str(word);
        }
        word.clear();
    };
    for c in d.message.chars() {
        if c == '`' {
            quoted = !quoted;
            word.clear();
            template.push(c);
        } else if !quoted && (c.is_ascii_alphanumeric() || c == '_') {
            word.push(c);
        } else if !quoted {
            flush(&mut template, &mut word);
            template.push(c);
        }
    }
    flush(&mut template, &mut word);
    (d.severity, d.code, d.suppressed, template)
}

/// What the report does with a visible diagnostic.
#[derive(Clone, Copy, PartialEq)]
enum Show {
    Print,
    /// Folded into a `... and N more like this` line.
    Alike,
    /// Cut by `--max`.
    Over,
}

/// Every error is printed; warnings fill what is left of `max`, in source order. With
/// `collapse`, diagnostics past the `ALIKE_SHOWN`th printed one of their key fold away and
/// don't count toward `max`.
fn plan(shown: &[&Diagnostic], keys: &[Key], max: usize, collapse: bool) -> Vec<Show> {
    let mut errors: BTreeMap<&Key, usize> = BTreeMap::new();
    for (d, k) in shown.iter().zip(keys) {
        if d.severity == Severity::Error {
            *errors.entry(k).or_default() += 1;
        }
    }
    let printed_errors: usize = errors
        .values()
        .map(|&n| if collapse { n.min(ALIKE_SHOWN) } else { n })
        .sum();
    let mut budget = max.saturating_sub(printed_errors);
    let mut seen: BTreeMap<&Key, usize> = BTreeMap::new();
    shown
        .iter()
        .zip(keys)
        .map(|(d, k)| {
            let n = seen.entry(k).or_default();
            if collapse && *n >= ALIKE_SHOWN {
                return Show::Alike;
            }
            if d.severity != Severity::Error {
                if budget == 0 {
                    return Show::Over;
                }
                budget -= 1;
            }
            *n += 1;
            Show::Print
        })
        .collect()
}

struct Selection {
    /// Indices into the diagnostics of the visible ones.
    idx: Vec<usize>,
    keys: Vec<Key>,
    plan: Vec<Show>,
}

/// Picks what to show and fills in the deferred help of the diagnostics that are printed.
fn select(diags: &mut [Diagnostic], opts: &Opts, collapse: bool) -> Selection {
    let idx: Vec<usize> = (0..diags.len())
        .filter(|&i| opts.show_suppressed || !diags[i].suppressed)
        .collect();
    let shown: Vec<&Diagnostic> = idx.iter().map(|&i| &diags[i]).collect();
    let keys: Vec<Key> = shown.iter().map(|d| key(d)).collect();
    let plan = plan(&shown, &keys, opts.max, collapse);
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
    Selection { idx, keys, plan }
}

pub fn text(
    name: &str,
    src: &str,
    diags: &mut [Diagnostic],
    opts: &Opts,
    output: Option<&Output>,
) -> String {
    let sel = select(diags, opts, true);
    let diags = &*diags;
    let name = term::line(name);
    let lines: Vec<&str> = src.lines().collect();
    let source = |out: &mut String, line: u32| {
        if let Some(l) = (line as usize).checked_sub(1).and_then(|i| lines.get(i)) {
            out.push_str(&format!("{line:>5} | {}\n", term::line(l)));
        }
    };
    let mut totals: BTreeMap<&Key, usize> = BTreeMap::new();
    for k in &sel.keys {
        *totals.entry(k).or_default() += 1;
    }
    let mut seen: BTreeMap<&Key, usize> = BTreeMap::new();
    let mut overflow = 0;
    let mut out = String::new();
    for ((&i, k), show) in sel.idx.iter().zip(&sel.keys).zip(&sel.plan) {
        match show {
            Show::Alike => continue,
            Show::Over => {
                overflow += 1;
                continue;
            }
            Show::Print => {}
        }
        let d = &diags[i];
        let n = seen.entry(k).or_default();
        *n += 1;
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
        if *n == ALIKE_SHOWN && totals[k] > ALIKE_SHOWN {
            out.push_str(&format!(
                "... and {} more like this\n",
                totals[k] - ALIKE_SHOWN
            ));
        }
    }
    if overflow > 0 {
        out.push_str(&format!("... and {overflow} more\n"));
    }
    let c = counts(diags);
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
    let sel = select(diags, opts, false);
    let listed: Vec<&Diagnostic> = sel
        .idx
        .iter()
        .zip(&sel.plan)
        .filter(|(_, s)| **s == Show::Print)
        .map(|(&i, _)| &diags[i])
        .collect();
    let c = counts(diags);
    let v = serde_json::json!({
        "schema": SCHEMA,
        "tool": env!("CARGO_PKG_VERSION"),
        "input": { "name": name, "sha256": format!("{:x}", Sha256::digest(src.as_bytes())) },
        "output": output.map(|o| serde_json::json!({ "path": o.path, "written": o.written })),
        "counts": { "error": c.errors, "warn": c.warns, "suppressed": c.suppressed },
        "truncated": listed.len() < sel.idx.len(),
        "metrics": opts.metrics,
        "diagnostics": listed,
    });
    format!("{v}\n")
}
