mod fmt_cmd;
mod measure;
mod output;
mod ref_cmd;
mod report;
mod route_cmd;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use kashika_diag::Severity;

#[derive(Parser)]
#[command(
    name = "kashika",
    version,
    about = "Diagrams you place exactly; kashika checks the geometry"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check a .kash file and report diagnostics
    Check(Common),
    /// Check a .kash file and render it to SVG or PNG
    Render {
        #[command(flatten)]
        common: Common,
        /// Output file (.svg or .png)
        #[arg(short, long)]
        output: PathBuf,
        /// Write output even when there are errors
        #[arg(long)]
        force: bool,
        /// Overlay rulers, element ids, anchor dots and diagnostic markers
        #[arg(long)]
        debug: bool,
    },
    /// Suggest ready-to-paste paths between two points; never edits the file
    Route(route_cmd::RouteArgs),
    /// Print the exact width and height of a label, with markup applied
    Measure(measure::MeasureArgs),
    /// Print the language reference, or one topic of it
    Ref {
        /// syntax, anchors, paths, shapes, styles, metrics, lints, examples, cli, or a diagnostic code
        topic: Option<String>,
        /// Wrap the page in a JSON envelope
        #[arg(long)]
        json: bool,
    },
    /// Rewrite a .kash file in canonical layout (whitespace only)
    Fmt {
        /// The .kash file to format
        file: String,
        /// Exit 1 if the file is not formatted, without rewriting it
        #[arg(long)]
        check: bool,
    },
}

struct RenderOpts {
    output: PathBuf,
    force: bool,
    debug: bool,
}

#[derive(Args)]
struct Common {
    /// Input .kash file, or `-` for stdin
    file: String,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    #[arg(long, value_enum, default_value_t = FailOn::Warn)]
    fail_on: FailOn,
    /// Maximum number of diagnostics to print
    #[arg(long, default_value_t = 50)]
    max: usize,
    /// File name to report for stdin input
    #[arg(long)]
    stdin_name: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FailOn {
    Error,
    Warn,
    Never,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let code = if e.use_stderr() { 3 } else { 0 };
            let _ = e.print();
            return ExitCode::from(code);
        }
    };
    match cli.cmd {
        Cmd::Check(common) => run(&common, None),
        Cmd::Fmt { file, check } => fmt_cmd::run(&file, check),
        Cmd::Render {
            common,
            output,
            force,
            debug,
        } => run(
            &common,
            Some(RenderOpts {
                output,
                force,
                debug,
            }),
        ),
        Cmd::Route(args) => route_cmd::run(&args),
        Cmd::Measure(args) => measure::run(&args),
        Cmd::Ref { topic, json } => ref_cmd::run(topic.as_deref(), json),
    }
}

fn run(c: &Common, render: Option<RenderOpts>) -> ExitCode {
    let (name, src) = match read_input(c) {
        Ok(v) => v,
        Err(msg) => return tool_error(c.format, &msg),
    };
    if let Some(r) = &render {
        if output::kind(&r.output).is_none() {
            return tool_error(c.format, "output must end in .svg or .png");
        }
    }
    let (scene, diags) = kashika_model::compile(&src);
    let errors = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warns = diags.len() - errors;
    let output = match &render {
        None => None,
        Some(r) => {
            let written = errors == 0 || r.force;
            if written {
                let overlay = kashika_render::Overlay {
                    error_markers: errors > 0,
                    debug: r.debug,
                };
                let svg = kashika_render::render_svg_with(&scene, &diags, overlay);
                let kind = output::kind(&r.output).expect("checked above");
                let bytes = match output::encode(kind, svg) {
                    Ok(b) => b,
                    Err(e) => {
                        return tool_error(
                            c.format,
                            &format!("cannot render {}: {e}", r.output.display()),
                        )
                    }
                };
                if let Err(e) = write_atomic(&r.output, &bytes) {
                    return tool_error(
                        c.format,
                        &format!("cannot write {}: {e}", r.output.display()),
                    );
                }
            }
            Some(report::Output {
                path: r.output.display().to_string(),
                written,
            })
        }
    };
    let report = match c.format {
        Format::Text => report::text(&name, &src, &diags, c.max, output.as_ref()),
        Format::Json => report::json(&name, &src, &diags, c.max, output.as_ref()),
    };
    print!("{report}");
    ExitCode::from(exit_code(c.fail_on, errors, warns))
}

fn read_input(c: &Common) -> Result<(String, String), String> {
    if c.file == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("cannot read stdin: {e}"))?;
        Ok((c.stdin_name.clone().unwrap_or_else(|| "<stdin>".into()), s))
    } else {
        let s =
            std::fs::read_to_string(&c.file).map_err(|e| format!("cannot read {}: {e}", c.file))?;
        Ok((c.file.clone(), s))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn exit_code(fail_on: FailOn, errors: usize, warns: usize) -> u8 {
    match fail_on {
        FailOn::Never => 0,
        _ if errors > 0 => 2,
        FailOn::Warn if warns > 0 => 1,
        _ => 0,
    }
}

fn tool_error(format: Format, msg: &str) -> ExitCode {
    match format {
        Format::Text => eprintln!("kashika: {msg}"),
        Format::Json => println!(
            "{}",
            serde_json::json!({ "schema": report::SCHEMA, "tool": env!("CARGO_PKG_VERSION"), "tool_error": msg })
        ),
    }
    ExitCode::from(3)
}
