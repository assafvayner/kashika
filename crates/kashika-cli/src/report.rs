use kashika_diag::{Diagnostic, Severity};
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "kashika.check/1";

pub struct Output {
    pub path: String,
    pub written: bool,
}

fn counts(diags: &[Diagnostic]) -> (usize, usize) {
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    (errors, diags.len() - errors)
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn text(
    name: &str,
    src: &str,
    diags: &[Diagnostic],
    max: usize,
    output: Option<&Output>,
) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = String::new();
    for d in diags.iter().take(max) {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warn => "warn",
        };
        out.push_str(&format!(
            "{name}:{}:{}: {sev}[{}]: {}\n",
            d.span.line, d.span.col, d.code, d.message
        ));
        if let Some(l) = (d.span.line as usize)
            .checked_sub(1)
            .and_then(|i| lines.get(i))
        {
            out.push_str(&format!("{:>5} | {l}\n", d.span.line));
        }
        for h in &d.help {
            out.push_str(&format!("      = help: {h}\n"));
        }
    }
    if diags.len() > max {
        out.push_str(&format!("... and {} more\n", diags.len() - max));
    }
    let (errors, warns) = counts(diags);
    out.push_str(&format!(
        "{}, {}",
        plural(errors, "error"),
        plural(warns, "warning")
    ));
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
    max: usize,
    output: Option<&Output>,
) -> String {
    let (errors, warns) = counts(diags);
    let v = serde_json::json!({
        "schema": SCHEMA,
        "tool": env!("CARGO_PKG_VERSION"),
        "input": { "name": name, "sha256": format!("{:x}", Sha256::digest(src.as_bytes())) },
        "output": output.map(|o| serde_json::json!({ "path": o.path, "written": o.written })),
        "counts": { "error": errors, "warn": warns, "suppressed": 0 },
        "truncated": diags.len() > max,
        "diagnostics": &diags[..diags.len().min(max)],
    });
    format!("{v}\n")
}
