use kashika_check::Metrics;
use kashika_diag::{Diagnostic, Severity};
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "kashika.check/1";

pub struct Output {
    pub path: String,
    pub written: bool,
}

pub struct Opts {
    pub max: usize,
    pub show_suppressed: bool,
    pub metrics: Option<Metrics>,
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

pub fn text(
    name: &str,
    src: &str,
    diags: &[Diagnostic],
    opts: &Opts,
    output: Option<&Output>,
) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let source = |out: &mut String, line: u32| {
        if let Some(l) = (line as usize).checked_sub(1).and_then(|i| lines.get(i)) {
            out.push_str(&format!("{line:>5} | {l}\n"));
        }
    };
    let shown = visible(diags, opts.show_suppressed);
    let mut out = String::new();
    for d in shown.iter().take(opts.max) {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warn => "warn",
        };
        let note = if d.suppressed { " (suppressed)" } else { "" };
        out.push_str(&format!(
            "{name}:{}:{}: {sev}[{}]: {}{note}\n",
            d.span.line, d.span.col, d.code, d.message
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
            out.push_str(&format!("      = help: {h}\n"));
        }
        if let Some(r) = &d.reason {
            out.push_str(&format!("      = reason: {r}\n"));
        }
    }
    if shown.len() > opts.max {
        out.push_str(&format!("... and {} more\n", shown.len() - opts.max));
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
    diags: &[Diagnostic],
    opts: &Opts,
    output: Option<&Output>,
) -> String {
    let shown = visible(diags, opts.show_suppressed);
    let c = counts(diags);
    let v = serde_json::json!({
        "schema": SCHEMA,
        "tool": env!("CARGO_PKG_VERSION"),
        "input": { "name": name, "sha256": format!("{:x}", Sha256::digest(src.as_bytes())) },
        "output": output.map(|o| serde_json::json!({ "path": o.path, "written": o.written })),
        "counts": { "error": c.errors, "warn": c.warns, "suppressed": c.suppressed },
        "truncated": shown.len() > opts.max,
        "metrics": opts.metrics,
        "diagnostics": &shown[..shown.len().min(opts.max)],
    });
    format!("{v}\n")
}
