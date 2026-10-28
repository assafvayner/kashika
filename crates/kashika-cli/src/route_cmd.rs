use std::io::Read;
use std::process::ExitCode;

use clap::Args;
use kashika_route::{num, Error, Request, Style, Suggestion};

use crate::Format;

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
    #[arg(long, default_value_t = 8.0)]
    radius: f64,
    /// Number of candidates to print
    #[arg(long, default_value_t = 3)]
    max: usize,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Allow file references outside the input file's directory
    #[arg(long)]
    allow_path: bool,
    /// File name to report for stdin input
    #[arg(long)]
    stdin_name: Option<String>,
}

pub fn run(a: &RouteArgs) -> ExitCode {
    let (name, src) = match read(&a.file, a.stdin_name.as_deref()) {
        Ok(v) => v,
        Err(msg) => return crate::tool_error(a.format, &msg),
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
                "expected `<from> <to>` and an optional style (corner, arc, bezier)",
            )
        }
    };
    let req = Request {
        src: &src,
        from,
        to,
        style,
        line: a.line.as_deref(),
        radius: a.radius,
        max: a.max,
        opts: &crate::options(&a.file, a.allow_path),
    };
    match kashika_route::suggest(&req) {
        Err(Error::Usage(msg)) => crate::tool_error(a.format, &msg),
        Err(Error::Diagnostics(diags)) => {
            match a.format {
                Format::Json => println!(
                    "{}",
                    serde_json::json!({
                        "schema": "kashika.route/1",
                        "tool": env!("CARGO_PKG_VERSION"),
                        "input": { "name": name },
                        "diagnostics": diags,
                    })
                ),
                Format::Text => {
                    for d in &diags {
                        println!("{name}: error[{}]: {}", d.code, d.message);
                        for h in &d.help {
                            println!("      = help: {h}");
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
                        "schema": "kashika.route/1",
                        "tool": env!("CARGO_PKG_VERSION"),
                        "input": { "name": name },
                        "from": s.from,
                        "to": s.to,
                        "style": s.style,
                        "candidates": s.candidates,
                    })
                ),
                Format::Text => print!("{}", text(&s)),
            }
            ExitCode::from(u8::from(s.candidates.is_empty()))
        }
    }
}

fn read(file: &str, stdin_name: Option<&str>) -> Result<(String, String), String> {
    if file == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("cannot read stdin: {e}"))?;
        Ok((stdin_name.unwrap_or("<stdin>").to_string(), s))
    } else {
        let s = std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?;
        Ok((file.to_string(), s))
    }
}

fn text(s: &Suggestion) -> String {
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
