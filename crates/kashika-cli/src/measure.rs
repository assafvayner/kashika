use std::process::ExitCode;

use clap::{Args, ValueEnum};

use crate::Format;

pub const SCHEMA: &str = "kashika.measure/1";

#[derive(Args)]
pub struct MeasureArgs {
    /// Texts to measure, with markup and `\n` escapes as in a .kash string; one result per text
    #[arg(required = true)]
    text: Vec<String>,
    /// Font size in px
    #[arg(long, default_value_t = 14.0)]
    size: f64,
    #[arg(long, value_enum, default_value_t = Weight::Normal)]
    weight: Weight,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Weight {
    Normal,
    Bold,
}

/// Applies the string escapes the lexer applies: `\n`, `\"` and `\\`. Markup escapes
/// such as `\*` stay for the markup parser.
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn run(a: &MeasureArgs) -> ExitCode {
    if !(a.size.is_finite() && a.size > 0.0) {
        return crate::tool_error(a.format, SCHEMA, "--size must be a positive number");
    }
    let bold = a.weight == Weight::Bold;
    let weight = if bold { "bold" } else { "normal" };
    let results: Vec<_> = a
        .text
        .iter()
        .map(|t| kashika_text::measure(&unescape(t), a.size, bold))
        .collect();
    let failed = results.iter().any(Result::is_err);
    let entry = |r: &Result<kashika_text::Block, kashika_text::MarkupError>| match r {
        Ok(b) => serde_json::json!({
            "width": round(b.width()),
            "height": round(b.height()),
            "lines": b.lines().len(),
        }),
        Err(e) => serde_json::json!({ "error": { "code": "parse", "message": e.to_string() } }),
    };
    match a.format {
        Format::Text => {
            for r in &results {
                match r {
                    Ok(b) => println!(
                        "width={} height={} lines={}",
                        round(b.width()),
                        round(b.height()),
                        b.lines().len()
                    ),
                    Err(e) => println!("error[parse]: {}", crate::term::line(&e.to_string())),
                }
            }
        }
        Format::Json => {
            let mut v = serde_json::json!({
                "schema": SCHEMA,
                "tool": env!("CARGO_PKG_VERSION"),
                "size": a.size,
                "weight": weight,
            });
            let fields = v.as_object_mut().expect("object");
            if let [only] = results.as_slice() {
                if let Some(o) = entry(only).as_object() {
                    fields.extend(o.clone());
                }
            } else {
                let each: Vec<_> = a
                    .text
                    .iter()
                    .zip(&results)
                    .map(|(t, r)| {
                        let mut e = entry(r);
                        e.as_object_mut()
                            .expect("object")
                            .insert("text".into(), t.clone().into());
                        e
                    })
                    .collect();
                fields.insert("results".into(), each.into());
            }
            println!("{v}");
        }
    }
    ExitCode::from(if failed { 2 } else { 0 })
}
