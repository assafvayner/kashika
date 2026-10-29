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
use kashika_diag::did_you_mean;
use kashika_geom::Rect;
use kashika_model::Scene;

#[derive(Parser)]
#[command(
    name = "kashika",
    version,
    about = "Diagrams you place exactly; kashika checks the geometry",
    after_help = "Exit codes: 0 clean; 1 warnings (check/render with --fail-on warn), `fmt --check` with changes, or `route`/`icons --search` with no result; 2 errors; 3 usage or I/O failure."
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
        /// Output file (.svg, .png, .jpg, .jpeg or .webp); an existing file is left untouched when there are errors, unless --force
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
    #[command(visible_aliases = ["references", "docs"])]
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
    Inspect(InspectArgs),
    /// Rewrite a .kash file in canonical layout (whitespace only)
    #[command(visible_alias = "format")]
    Fmt {
        /// The .kash file to format, or `-` to format stdin to stdout
        file: String,
        /// Exit 1 if the file is not formatted, without rewriting it
        #[arg(long)]
        check: bool,
        /// File name to report for stdin input
        #[arg(long)]
        stdin_name: Option<String>,
    },
}

#[derive(Args)]
struct InspectArgs {
    /// Input .kash file, or `-` for stdin
    file: String,
    /// Only show these ids
    ids: Vec<String>,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// File name to report for stdin input
    #[arg(long)]
    stdin_name: Option<String>,
    /// Allow file references outside the input file's directory
    #[arg(long)]
    allow_path: bool,
    /// Allow this one file outside the input file's directory (repeatable; relative to the current directory)
    #[arg(long, value_name = "FILE")]
    include: Vec<PathBuf>,
    /// List the endpoints on each shape side and the lines that use them
    #[arg(long)]
    ports: bool,
    /// Free horizontal and vertical intervals inside the rectangle x1,y1,x2,y2; without ids, prints only these
    #[arg(
        long,
        value_name = "X1,Y1,X2,Y2",
        allow_hyphen_values = true,
        value_parser = parse_rect
    )]
    corridors: Option<Rect>,
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
    /// Allow this one file outside the input file's directory (repeatable; relative to the current directory)
    #[arg(long, value_name = "FILE")]
    include: Vec<PathBuf>,
    /// Also list suppressed findings
    #[arg(long)]
    show_suppressed: bool,
    /// Also print the number of findings per diagnostic code
    #[arg(long)]
    summary: bool,
    /// Only report warnings that involve these ids (comma-separated); errors are always reported
    #[arg(long, value_delimiter = ',')]
    focus: Vec<String>,
}

impl Common {
    fn report_opts<'a>(
        &self,
        metrics: Option<kashika_check::Metrics>,
        scene: Option<&'a Scene>,
        omitted: &'a [kashika_check::Omitted],
    ) -> report::Opts<'a> {
        report::Opts {
            max: self.max,
            show_suppressed: self.show_suppressed,
            summary: self.summary,
            metrics,
            scene,
            omitted,
        }
    }
}

pub(crate) fn icon_path() -> Vec<PathBuf> {
    std::env::var_os("KASHIKA_ICON_PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default()
}

/// The sandbox for `file`. Each `--include` must name an existing file; it is canonicalized
/// relative to the current directory.
pub(crate) fn options(
    file: &str,
    allow_path: bool,
    include: &[PathBuf],
) -> Result<kashika_model::Options, String> {
    let include = include
        .iter()
        .map(|p| match p.canonicalize() {
            Ok(c) if c.is_file() => Ok(c),
            Ok(_) => Err(format!(
                "--include `{}` is not a file; include each file, or pass --allow-path",
                p.display()
            )),
            Err(e) => Err(format!("--include `{}`: {e}", p.display())),
        })
        .collect::<Result<_, _>>()?;
    Ok(kashika_model::Options {
        path: (file != "-").then(|| PathBuf::from(file)),
        allow_path,
        icon_path: icon_path(),
        include,
    })
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
            let args: Vec<String> = std::env::args().skip(1).collect();
            if args.first().is_some_and(|c| c == "route")
                && args.iter().any(|a| a.ends_with("->") && a.starts_with('-'))
            {
                eprintln!("help: {}", route_cmd::NO_ARROW);
            }
            return ExitCode::from(code);
        }
    };
    match cli.cmd {
        Cmd::Check(common) => run(&common, None),
        Cmd::Fmt {
            file,
            check,
            stdin_name,
        } => fmt_cmd::run(&file, check, stdin_name.as_deref()),
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
        Cmd::Inspect(args) => run_inspect(&args),
        Cmd::Route(args) => route_cmd::run(&args),
        Cmd::Measure(args) => measure::run(&args),
        Cmd::Icons(args) => icons::run(&args),
        Cmd::Ref { topic, json } => ref_cmd::run(topic.as_deref(), json),
    }
}

fn run(c: &Common, render: Option<RenderOpts>) -> ExitCode {
    let fail = |msg: &str| tool_error(c.format, report::SCHEMA, msg);
    let (name, src) = match read_input(&c.file, c.stdin_name.as_deref()) {
        Ok(v) => v,
        Err(msg) => return fail(&msg),
    };
    if let Some(r) = &render {
        if output::kind(&r.output).is_none() {
            return fail(&format!("output must end in {}", output::SUPPORTED));
        }
    }
    let opts = match options(&c.file, c.allow_path, &c.include) {
        Ok(o) => o,
        Err(msg) => return fail(&msg),
    };
    if let Some(diags) = kashika_model::compile_library_with(&src, &opts) {
        if render.is_some() {
            return fail(&format!(
                "{name} is a library (no `diagram` statement); render a diagram that imports it"
            ));
        }
        return report_library(c, &name, &src, diags);
    }
    let (scene, mut diags) = kashika_model::compile_with(&src, &opts);
    if !c.focus.is_empty() {
        if let Err(msg) = known_ids(&scene_ids(&scene), &c.focus, "--focus") {
            return fail(&msg);
        }
        retain_focus(&mut diags, &c.focus);
    }
    let focus = |d: &kashika_diag::Diagnostic| {
        c.focus.is_empty() || d.elements.iter().any(|e| c.focus.contains(&e.id))
    };
    let found = kashika_check::check_bounded(&scene, c.max.max(report::KEEP), &focus);
    let metrics = found.metrics(&scene);
    diags.extend(found.diags);
    kashika_diag::sort(&mut diags);
    let omitted = found.omitted;
    let report::Counts { errors, warns, .. } = report::counts(&diags, &omitted);
    let output = match &render {
        None => None,
        Some(r) => {
            let written = errors == 0 || r.force;
            if written {
                let overlay = kashika_render::Overlay {
                    error_markers: errors > 0,
                    debug: r.debug,
                };
                let marked = report::visible(&diags, c.show_suppressed);
                let svg = kashika_render::render_svg_with(&scene, marked, overlay);
                let kind = output::kind(&r.output).expect("checked above");
                let bytes = match output::encode(kind, svg) {
                    Ok(b) => b,
                    Err(e) => {
                        return fail(&format!("cannot render {}: {e}", r.output.display()));
                    }
                };
                if let Err(e) = write_atomic(&r.output, &bytes) {
                    return fail(&format!("cannot write {}: {e}", r.output.display()));
                }
            }
            Some(report::Output {
                path: r.output.display().to_string(),
                written,
            })
        }
    };
    let opts = c.report_opts(Some(metrics), Some(&scene), &omitted);
    let report = match c.format {
        Format::Text => report::text(&name, &src, &mut diags, &opts, output.as_ref()),
        Format::Json => report::json(&name, &src, &mut diags, &opts, output.as_ref()),
    };
    print!("{report}");
    ExitCode::from(exit_code(c.fail_on, errors, warns))
}

fn retain_focus(diags: &mut Vec<kashika_diag::Diagnostic>, focus: &[String]) {
    diags.retain(|d| {
        d.severity == kashika_diag::Severity::Error
            || d.elements.iter().any(|e| focus.contains(&e.id))
    });
}

fn report_library(
    c: &Common,
    name: &str,
    src: &str,
    mut diags: Vec<kashika_diag::Diagnostic>,
) -> ExitCode {
    if !c.focus.is_empty() {
        let (file, _) = kashika_syntax::parse(src);
        let defined: Vec<String> = file
            .stmts
            .into_iter()
            .filter_map(|s| match s {
                kashika_syntax::ast::Stmt::Shape(s) => Some(s.name.name),
                _ => None,
            })
            .collect();
        if let Err(msg) = known_ids(&defined, &c.focus, "--focus") {
            return tool_error(c.format, report::SCHEMA, &msg);
        }
        retain_focus(&mut diags, &c.focus);
    }
    let opts = c.report_opts(None, None, &[]);
    let report = match c.format {
        Format::Text => report::text(name, src, &mut diags, &opts, None),
        Format::Json => report::json(name, src, &mut diags, &opts, None),
    };
    print!("{report}");
    let report::Counts { errors, warns, .. } = report::counts(&diags, &[]);
    ExitCode::from(exit_code(c.fail_on, errors, warns))
}

fn run_inspect(a: &InspectArgs) -> ExitCode {
    let fail = |msg: &str| tool_error(a.format, inspect::SCHEMA, msg);
    let (name, src) = match read_input(&a.file, a.stdin_name.as_deref()) {
        Ok(v) => v,
        Err(msg) => return fail(&msg),
    };
    let opts = match options(&a.file, a.allow_path, &a.include) {
        Ok(o) => o,
        Err(msg) => return fail(&msg),
    };
    let (scene, mut diags) = kashika_model::compile_with(&src, &opts);
    diags.retain(|d| !d.suppressed);
    let lints: usize = kashika_check::check_bounded(&scene, 0, &|d| !d.suppressed)
        .omitted
        .iter()
        .map(|o| o.count)
        .sum();
    if let Err(msg) = known_ids(&scene_ids(&scene), &a.ids, "inspect") {
        return fail(&msg);
    }
    let req = inspect::Request {
        ids: &a.ids,
        ports: a.ports,
        corridors: a.corridors,
    };
    let data = inspect::build(&name, &src, &scene, &diags, lints, &req);
    match a.format {
        Format::Text => print!("{}", term::text(&inspect::text(&data))),
        Format::Json => match serde_json::to_string(&data) {
            Ok(json) => println!("{json}"),
            Err(e) => return fail(&format!("cannot serialize inspect output: {e}")),
        },
    }
    ExitCode::from(0)
}

fn parse_rect(s: &str) -> Result<Rect, String> {
    let bad = || format!("expected finite numbers x1,y1,x2,y2, got `{s}`");
    let v: Vec<f64> = s
        .split(',')
        .map(|x| x.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|_| bad())?;
    match v.as_slice() {
        &[x1, y1, x2, y2] if v.iter().all(|n| n.is_finite()) => Ok(Rect::new(
            x1.min(x2),
            y1.min(y2),
            (x2 - x1).abs(),
            (y2 - y1).abs(),
        )),
        _ => Err(bad()),
    }
}

pub(crate) fn non_negative(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    if v.is_finite() && v >= 0.0 {
        Ok(v)
    } else {
        Err(format!("`{s}` must be a finite number, 0 or more"))
    }
}

pub(crate) fn at_least_one(s: &str) -> Result<usize, String> {
    match s.parse::<usize>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err(format!("`{s}` must be a whole number, 1 or more")),
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

fn known_ids(all: &[String], ids: &[String], what: &str) -> Result<(), String> {
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

pub(crate) fn read_input(file: &str, stdin_name: Option<&str>) -> Result<(String, String), String> {
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

pub(crate) fn tool_error(format: Format, schema: &str, msg: &str) -> ExitCode {
    match format {
        Format::Text => eprintln!("kashika: {}", term::text(msg)),
        Format::Json => println!(
            "{}",
            serde_json::json!({ "schema": schema, "tool": env!("CARGO_PKG_VERSION"), "tool_error": msg })
        ),
    }
    ExitCode::from(3)
}
