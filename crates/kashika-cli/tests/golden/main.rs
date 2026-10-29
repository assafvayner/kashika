mod case;
mod coverage;
mod dump;
mod review;

use std::path::Path;
use std::process::Command;

use case::{Case, OUTPUTS, Suite};
use libtest_mimic::{Arguments, Failed, Trial};
use serde_json::Value;

/// One expected file: its name in the case directory and its bytes.
struct Output {
    name: String,
    bytes: Vec<u8>,
}

impl Output {
    fn text(name: &str, s: String) -> Output {
        Output {
            name: name.to_string(),
            bytes: s.into_bytes(),
        }
    }
}

fn bless() -> bool {
    std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1")
}

fn main() {
    let args = Arguments::from_args();
    let root = case::golden_root();
    let mut trials = Vec::new();
    for dir in case::discover(&root) {
        let name = dir
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let root = root.clone();
        trials.push(Trial::test(name, move || {
            let case = Case::load(&root, &dir)?;
            run_case(&case)
        }));
    }
    let cases = libtest_mimic::run(&args, trials);
    let coverage = libtest_mimic::run(&args, coverage::trials());
    if let Some(page) = review::write() {
        eprintln!("review changed renders side by side: {}", page.display());
    }
    if cases.has_failed() || coverage.has_failed() {
        std::process::exit(101);
    }
}

fn run_case(case: &Case) -> Result<(), Failed> {
    let outputs = match case.suite {
        Suite::Parse => run_parse(case)?,
        Suite::Render => run_render(case)?,
    };
    check_outputs(case, &outputs)
}

fn run_parse(case: &Case) -> Result<Vec<Output>, String> {
    let input = case.dir.join("input.kash");
    let (file, _) = kashika_syntax::parse(&case.src);
    let opts = kashika_model::Options {
        path: Some(input),
        allow_path: case.allow_path,
        icon_path: case.icon_path.iter().map(|p| case.dir.join(p)).collect(),
        include: case.include.iter().map(|p| case.dir.join(p)).collect(),
    };
    let (scene, diags) = match kashika_model::compile_library_with(&case.src, &opts) {
        Some(diags) => (None, diags),
        None => {
            let (scene, mut diags) = kashika_model::compile_with(&case.src, &opts);
            diags.extend(kashika_check::check(&scene));
            kashika_diag::sort(&mut diags);
            (Some(scene), diags)
        }
    };
    let diags: Vec<Value> = diags
        .iter()
        .map(|d| serde_json::to_value(d).expect("diagnostics serialize"))
        .collect();

    if let Ok(once) = kashika_syntax::format(&case.src) {
        let twice = kashika_syntax::format(&once)
            .map_err(|_| format!("{}: fmt output no longer parses", case.name))?;
        if once != twice {
            return Err(format!(
                "{}: fmt is not idempotent on this input",
                case.name
            ));
        }
    }

    let mut out = vec![Output::text("diagnostics.txt", dump::diagnostics(&diags))];
    if case.dump_ast {
        out.push(Output::text("ast.txt", dump::ast(&file)));
    }
    if case.dump_scene {
        let scene = scene.ok_or_else(|| format!("{}: a library has no scene", case.name))?;
        out.push(Output::text("scene.json", dump::scene(&scene)));
    }
    Ok(out)
}

fn kashika(case: &Case, out: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kashika"));
    cmd.current_dir(&case.dir)
        .env_remove("KASHIKA_ICON_PATH")
        .args(["render", "input.kash", "-o"])
        .arg(out);
    if let Some(p) = &case.icon_path {
        cmd.env("KASHIKA_ICON_PATH", case.dir.join(p));
    }
    for p in &case.include {
        cmd.arg("--include").arg(case.dir.join(p));
    }
    for (on, flag) in [
        (case.force, "--force"),
        (case.debug, "--debug"),
        (case.allow_path, "--allow-path"),
    ] {
        if on {
            cmd.arg(flag);
        }
    }
    cmd
}

fn run_render(case: &Case) -> Result<Vec<Output>, String> {
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut formats = vec!["svg".to_string()];
    formats.extend(case.raster.iter().cloned());
    for ext in formats {
        let path = tmp.path().join(format!("out.{ext}"));
        let mut cmd = kashika(case, &path);
        if ext == "svg" {
            cmd.args(["--format", "json", "--max", "1000", "--show-suppressed"]);
        }
        let run = cmd.output().map_err(|e| e.to_string())?;
        let code = run.status.code().unwrap_or(-1);
        let ok = matches!(code, 0 | 1) || (case.force && code == 2);
        if !ok || !path.is_file() {
            return Err(format!(
                "{}: `kashika render -o out.{ext}` exited {code} without writing output\nstdout:\n{}\nstderr:\n{}",
                case.name,
                String::from_utf8_lossy(&run.stdout),
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        if ext == "svg" {
            let report: Value = serde_json::from_slice(&run.stdout).map_err(|e| e.to_string())?;
            let diags = report["diagnostics"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            out.push(Output::text("diagnostics.txt", dump::diagnostics(&diags)));
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        out.push(Output {
            name: format!("out.{ext}"),
            bytes,
        });
    }
    Ok(out)
}

/// Compares (or, with `KASHIKA_BLESS=1`, writes) every output, and flags expected files
/// the case no longer produces. Mismatching outputs are saved under the target directory.
fn check_outputs(case: &Case, outputs: &[Output]) -> Result<(), Failed> {
    let stale: Vec<&str> = OUTPUTS
        .iter()
        .copied()
        .filter(|n| case.dir.join(n).exists() && !outputs.iter().any(|o| o.name == *n))
        .collect();
    let svg = outputs.iter().find(|o| o.name == "out.svg");
    let expected_svg = svg.and_then(|_| std::fs::read(case.dir.join("out.svg")).ok());
    let review = |blessed: bool| {
        if let Some(svg) = svg {
            review::record(review::Entry {
                name: case.name.clone(),
                description: case.description.clone(),
                expected: expected_svg.clone(),
                actual: svg.bytes.clone(),
                blessed,
            });
        }
    };
    if bless() {
        if svg.is_some_and(|o| expected_svg.as_ref() != Some(&o.bytes)) {
            review(true);
        }
        for o in outputs {
            std::fs::write(case.dir.join(&o.name), &o.bytes).map_err(|e| e.to_string())?;
        }
        for n in stale {
            std::fs::remove_file(case.dir.join(n)).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let hint = format!(
        "if the change is intended, run: KASHIKA_BLESS=1 cargo test -p kashika-cli --test golden -- {}",
        case.name
    );
    let mut problems = Vec::new();
    for n in stale {
        problems.push(format!("{n} is stale: this case no longer produces it"));
    }
    let actual_dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("golden-actual")
        .join(&case.name);
    for o in outputs {
        let expected = match std::fs::read(case.dir.join(&o.name)) {
            Ok(b) => b,
            Err(_) => {
                problems.push(format!("{} is missing", o.name));
                continue;
            }
        };
        if expected == o.bytes {
            continue;
        }
        std::fs::create_dir_all(&actual_dir).map_err(|e| e.to_string())?;
        let saved = actual_dir.join(&o.name);
        std::fs::write(&saved, &o.bytes).map_err(|e| e.to_string())?;
        match (
            std::str::from_utf8(&expected),
            std::str::from_utf8(&o.bytes),
        ) {
            (Ok(e), Ok(a)) => problems.push(format!(
                "{} differs (actual saved to {}):\n{}",
                o.name,
                saved.display(),
                similar::TextDiff::from_lines(e, a)
                    .unified_diff()
                    .context_radius(3)
                    .header("expected", "actual")
            )),
            _ => problems.push(format!(
                "{} differs: expected {} bytes, got {} (actual saved to {})",
                o.name,
                expected.len(),
                o.bytes.len(),
                saved.display()
            )),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        review(false);
        Err(format!("{}\n{}\n{hint}", case.name, problems.join("\n")).into())
    }
}
