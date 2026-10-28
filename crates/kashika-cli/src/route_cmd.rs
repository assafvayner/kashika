use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use kashika_route::{Error, Request, Style, Suggestion, num};

use crate::{Format, term};

pub const SCHEMA: &str = "kashika.route/1";

#[derive(Args)]
pub struct RouteArgs {
    /// Input .kash file, or `-` for stdin
    file: String,
    /// `<from> <to>` points (e.g. `app1.bottom pg.left@0.25`), then an optional style:
    /// corner (default), arc or bezier. With `--line`, the points may be omitted.
    #[arg(num_args = 0..=3)]
    rest: Vec<String>,
    /// Reroute this existing line, keeping its id and attributes
    #[arg(long)]
    line: Option<String>,
    /// Corner radius for the `arc` style
    #[arg(long, default_value_t = 8.0, allow_hyphen_values = true, value_parser = crate::non_negative)]
    radius: f64,
    /// Number of candidates to print
    #[arg(long, default_value_t = 3, value_parser = crate::at_least_one)]
    max: usize,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Allow file references outside the input file's directory
    #[arg(long)]
    allow_path: bool,
    /// Allow this one file outside the input file's directory (repeatable; relative to the current directory)
    #[arg(long, value_name = "FILE")]
    include: Vec<PathBuf>,
    /// File name to report for stdin input
    #[arg(long)]
    stdin_name: Option<String>,
}

pub fn run(a: &RouteArgs) -> ExitCode {
    let (name, src) = match crate::read_input(&a.file, a.stdin_name.as_deref()) {
        Ok(v) => v,
        Err(msg) => return crate::tool_error(a.format, SCHEMA, &msg),
    };
    let mut rest: Vec<&str> = a.rest.iter().map(String::as_str).collect();
    let style = match rest.last().copied() {
        Some("corner") => Some(Style::Corner),
        Some("arc") => Some(Style::Arc),
        Some("bezier") => Some(Style::Bezier),
        _ => None,
    };
    if style.is_some() {
        rest.pop();
    }
    let style = style.unwrap_or(Style::Corner);
    let (from, to) = match rest.as_slice() {
        [] => (None, None),
        [f, t] => (Some(*f), Some(*t)),
        _ => {
            return crate::tool_error(
                a.format,
                SCHEMA,
                "expected `<from> <to>` and an optional style (corner, arc, bezier)",
            );
        }
    };
    let opts = match crate::options(&a.file, a.allow_path, &a.include) {
        Ok(o) => o,
        Err(msg) => return crate::tool_error(a.format, SCHEMA, &msg),
    };
    let req = Request {
        src: &src,
        from,
        to,
        style,
        line: a.line.as_deref(),
        radius: a.radius,
        max: a.max,
        opts: &opts,
    };
    match kashika_route::suggest(&req) {
        Err(Error::Usage(msg)) => crate::tool_error(a.format, SCHEMA, &msg),
        Err(Error::Diagnostics(diags)) => {
            match a.format {
                Format::Json => println!(
                    "{}",
                    serde_json::json!({
                        "schema": SCHEMA,
                        "tool": env!("CARGO_PKG_VERSION"),
                        "input": { "name": name },
                        "diagnostics": diags,
                    })
                ),
                Format::Text => {
                    let name = term::line(&name);
                    for d in &diags {
                        println!("{name}: error[{}]: {}", d.code, term::line(&d.message));
                        for h in &d.help {
                            println!("      = help: {}", term::line(h));
                        }
                    }
                }
            }
            ExitCode::from(2)
        }
        Ok(s) => {
            match a.format {
                Format::Json => println!(
                    "{}",
                    serde_json::json!({
                        "schema": SCHEMA,
                        "tool": env!("CARGO_PKG_VERSION"),
                        "input": { "name": name },
                        "from": s.from,
                        "to": s.to,
                        "style": s.style,
                        "candidates": s.candidates,
                        "existing_lines": s.existing_lines,
                    })
                ),
                Format::Text => print!("{}", term::text(&text(&s))),
            }
            ExitCode::from(u8::from(s.candidates.is_empty()))
        }
    }
}

fn text(s: &Suggestion) -> String {
    let mut out = candidates_text(s);
    if let Some(id) = s.existing_lines.first() {
        out.push_str(&format!(
            "hint: line `{id}` already connects these; pass --line {id} to reroute it instead of avoiding it\n"
        ));
    }
    out
}

fn candidates_text(s: &Suggestion) -> String {
    if s.candidates.is_empty() {
        return "no candidate path avoids every box with at most 4 elbows (or 1 curve); try other anchors, or move a box\n".into();
    }
    let mut out = String::new();
    for (i, c) in s.candidates.iter().enumerate() {
        let mut crossings = format!("crossings={}", c.crossings);
        if !c.crossed.is_empty() {
            let list: Vec<String> = c
                .crossed
                .iter()
                .map(|x| format!("{} at ({},{})", x.line, num(x.at.x), num(x.at.y)))
                .collect();
            crossings.push_str(&format!(" ({})", list.join(", ")));
        }
        out.push_str(&format!(
            "#{}  {crossings} near={} segments={} length={}px",
            i + 1,
            c.near,
            c.segments,
            num(c.length)
        ));
        if let Some(k) = &c.corridor {
            out.push_str(&format!(
                "  via corridor {} {}..{}",
                k.axis,
                num(k.band[0]),
                num(k.band[1])
            ));
        }
        out.push_str(&format!("\n    {}\n", c.statement));
    }
    out
}
