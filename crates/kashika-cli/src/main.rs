mod fmt_cmd;
mod icons;
mod inspect;
mod measure;
mod output;
mod ref_cmd;
mod report;
mod route_cmd;
mod term;

use std::hash::BuildHasher;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use kashika_diag::{Severity, did_you_mean};
use kashika_geom::Rect;
use kashika_model::Scene;

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
    /// Check a .kash file and render it to SVG, PNG, JPEG or WebP
    Render {
        #[command(flatten)]
        common: Common,
        /// Output file (.svg, .png, .jpg, .jpeg or .webp)
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
    /// List bundled icons and the importable icons and shapes in directories
    Icons(icons::IconsArgs),
    /// Print the resolved geometry: shapes, anchors, lines, labels and the placement graph
    Inspect {
        #[command(flatten)]
        common: Common,
        /// Only show these ids
        ids: Vec<String>,
        /// List the endpoints on each shape side and the lines that use them
        #[arg(long)]
        ports: bool,
        /// Free horizontal and vertical intervals inside the rectangle x1,y1,x2,y2
        #[arg(long, value_name = "X1,Y1,X2,Y2", allow_hyphen_values = true)]
        corridors: Option<String>,
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
    /// Allow file references outside the input file's directory
    #[arg(long)]
    allow_path: bool,
    /// Also list suppressed findings
    #[arg(long)]
    show_suppressed: bool,
    /// Only report warnings that involve these ids (comma-separated); errors are always reported
    #[arg(long, value_delimiter = ',')]
    focus: Vec<String>,
}

pub(crate) fn icon_path() -> Vec<PathBuf> {
    std::env::var_os("KASHIKA_ICON_PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default()
}

pub(crate) fn options(file: &str, allow_path: bool) -> kashika_model::Options {
    kashika_model::Options {
        path: (file != "-").then(|| PathBuf::from(file)),
        allow_path,
        icon_path: icon_path(),
    }
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
        Cmd::Inspect {
            common,
            ids,
            ports,
            corridors,
        } => run_inspect(&common, &ids, ports, corridors.as_deref()),
        Cmd::Route(args) => route_cmd::run(&args),
        Cmd::Measure(args) => measure::run(&args),
        Cmd::Icons(args) => icons::run(&args),
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
            return tool_error(
                c.format,
                &format!("output must end in {}", output::SUPPORTED),
            );
        }
    }
    let opts = options(&c.file, c.allow_path);
    if let Some(diags) = kashika_model::compile_library_with(&src, &opts) {
        if render.is_some() {
            return tool_error(
                c.format,
                &format!(
                    "{name} is a library (no `diagram` statement); render a diagram that imports it"
                ),
            );
        }
        return report_library(c, &name, &src, diags);
    }
    let (scene, mut diags) = kashika_model::compile_with(&src, &opts);
    diags.extend(kashika_check::check_deferred(&scene));
    kashika_diag::sort(&mut diags);
    let metrics = kashika_check::metrics(&scene, &diags);
    if !c.focus.is_empty() {
        if let Err(msg) = known_ids(&scene, &c.focus, "--focus") {
            return tool_error(c.format, &msg);
        }
        diags.retain(|d| {
            d.severity == Severity::Error || d.elements.iter().any(|e| c.focus.contains(&e.id))
        });
    }
    let report::Counts { errors, warns, .. } = report::counts(&diags);
    let output = match &render {
        None => None,
        Some(r) => {
            let written = errors == 0 || r.force;
            if written {
                let overlay = kashika_render::Overlay {
                    error_markers: errors > 0,
                    debug: r.debug,
                };
                let marked: Vec<_> = report::visible(&diags, c.show_suppressed)
                    .into_iter()
                    .cloned()
                    .collect();
                let svg = kashika_render::render_svg_with(&scene, &marked, overlay);
                let kind = output::kind(&r.output).expect("checked above");
                let bytes = match output::encode(kind, svg) {
                    Ok(b) => b,
                    Err(e) => {
                        return tool_error(
                            c.format,
                            &format!("cannot render {}: {e}", r.output.display()),
                        );
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
    let opts = report::Opts {
        max: c.max,
        show_suppressed: c.show_suppressed,
        metrics: Some(metrics),
        scene: Some(&scene),
    };
    let report = match c.format {
        Format::Text => report::text(&name, &src, &mut diags, &opts, output.as_ref()),
        Format::Json => report::json(&name, &src, &mut diags, &opts, output.as_ref()),
    };
    print!("{report}");
    ExitCode::from(exit_code(c.fail_on, errors, warns))
}

fn report_library(
    c: &Common,
    name: &str,
    src: &str,
    mut diags: Vec<kashika_diag::Diagnostic>,
) -> ExitCode {
    let opts = report::Opts {
        max: c.max,
        show_suppressed: c.show_suppressed,
        metrics: None,
        scene: None,
    };
    let report = match c.format {
        Format::Text => report::text(name, src, &mut diags, &opts, None),
        Format::Json => report::json(name, src, &mut diags, &opts, None),
    };
    print!("{report}");
    let report::Counts { errors, warns, .. } = report::counts(&diags);
    ExitCode::from(exit_code(c.fail_on, errors, warns))
}

fn run_inspect(c: &Common, ids: &[String], ports: bool, corridors: Option<&str>) -> ExitCode {
    let (name, src) = match read_input(c) {
        Ok(v) => v,
        Err(msg) => return tool_error(c.format, &msg),
    };
    let (scene, mut diags) = kashika_model::compile_with(&src, &options(&c.file, c.allow_path));
    diags.extend(kashika_check::check_deferred(&scene));
    diags.retain(|d| !d.suppressed);
    if let Err(msg) = known_ids(&scene, ids, "inspect") {
        return tool_error(c.format, &msg);
    }
    let corridors = match corridors.map(parse_rect).transpose() {
        Ok(r) => r,
        Err(msg) => return tool_error(c.format, &msg),
    };
    let req = inspect::Request {
        ids,
        ports,
        corridors,
    };
    let data = inspect::build(&name, &src, &scene, &diags, &req);
    match c.format {
        Format::Text => print!("{}", term::text(&inspect::text(&data))),
        Format::Json => println!("{}", serde_json::to_string(&data).unwrap_or_default()),
    }
    ExitCode::from(0)
}

fn parse_rect(s: &str) -> Result<Rect, String> {
    let bad = || format!("--corridors expects x1,y1,x2,y2, got `{s}`");
    let v: Vec<f64> = s
        .split(',')
        .map(|x| x.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|_| bad())?;
    match v.as_slice() {
        &[x1, y1, x2, y2] => Ok(Rect::new(
            x1.min(x2),
            y1.min(y2),
            (x2 - x1).abs(),
            (y2 - y1).abs(),
        )),
        _ => Err(bad()),
    }
}

/// Every id in the scene, with `line@L<n>`/`text@L<n>` for unnamed lines and texts.
fn scene_ids(scene: &Scene) -> Vec<String> {
    scene
        .shapes
        .iter()
        .map(|s| s.id.clone())
        .chain(scene.junctions.iter().map(|j| j.id.clone()))
        .chain(scene.lines.iter().map(|l| l.id.clone()))
        .chain(scene.texts.iter().map(kashika_check::ctx::text_id))
        .collect()
}

fn known_ids(scene: &Scene, ids: &[String], what: &str) -> Result<(), String> {
    let all = scene_ids(scene);
    for id in ids {
        if !all.contains(id) {
            let hint = did_you_mean(id, all.iter().map(String::as_str))
                .map(|s| format!("; did you mean `{s}`?"))
                .unwrap_or_default();
            return Err(format!("unknown id `{id}` in {what}{hint}"));
        }
    }
    Ok(())
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

fn resolve_link(path: &Path) -> PathBuf {
    if let Ok(p) = std::fs::canonicalize(path) {
        return p;
    }
    match std::fs::read_link(path) {
        Ok(t) => path.parent().unwrap_or(Path::new("")).join(t),
        Err(_) => path.to_path_buf(),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let path = &resolve_link(path);
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nonce = std::hash::RandomState::new().hash_one(std::process::id());
    let tmp = dir.join(format!(".{name}.{nonce:016x}.tmp"));
    let mut f = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    let result = (|| {
        f.write_all(bytes)?;
        if let Ok(meta) = std::fs::metadata(path) {
            f.set_permissions(meta.permissions())?;
        }
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    } else {
        #[cfg(unix)]
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
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
        Format::Text => eprintln!("kashika: {}", term::text(msg)),
        Format::Json => println!(
            "{}",
            serde_json::json!({ "schema": report::SCHEMA, "tool": env!("CARGO_PKG_VERSION"), "tool_error": msg })
        ),
    }
    ExitCode::from(3)
}
