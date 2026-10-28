use std::process::ExitCode;

use clap::{Args, ValueEnum};

use crate::Format;

pub const SCHEMA: &str = "kashika.measure/1";

#[derive(Args)]
pub struct MeasureArgs {
    /// Text to measure, with markup and `\n` escapes as in a .kash string
    text: String,
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
        return crate::tool_error(a.format, "--size must be a positive number");
    }
    let text = unescape(&a.text);
    let bold = a.weight == Weight::Bold;
    match kashika_text::measure(&text, a.size, bold) {
        Ok(b) => {
            let (w, h, n) = (round(b.width), round(b.height), b.lines.len());
            match a.format {
                Format::Text => println!("width={w} height={h} lines={n}"),
                Format::Json => println!(
                    "{}",
                    serde_json::json!({
                        "schema": SCHEMA,
                        "tool": env!("CARGO_PKG_VERSION"),
                        "size": a.size,
                        "weight": if bold { "bold" } else { "normal" },
                        "width": w,
                        "height": h,
                        "lines": n,
                    })
                ),
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            match a.format {
                Format::Text => println!("error[parse]: {}", crate::term::line(&e.to_string())),
                Format::Json => println!(
                    "{}",
                    serde_json::json!({
                        "schema": SCHEMA,
                        "tool": env!("CARGO_PKG_VERSION"),
                        "error": { "code": "parse", "message": e.to_string() },
                    })
                ),
            }
            ExitCode::from(2)
        }
    }
}
