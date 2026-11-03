//! CLI snapshots: every directory under `tests/cli-snapshots/` runs `kashika` once and
//! compares its stdout, stderr and exit code with `expect.stdout`, `expect.stderr` and
//! `expect.code`. `KASHIKA_BLESS=1` writes them instead.
//!
//! A case holds `argv` (one argument per line), optionally `stdin` (fed to the process;
//! without it stdin is empty) and `setup` (lines of `copy <from> <to>`, `mkdir <path>` or
//! `chmod <octal> <path>`, run before the command; a case with `chmod` runs on Unix only).
//! In `argv` and `setup`, `$FIXTURES` is `tests/fixtures`, `$TMP` a fresh temporary
//! directory and `$CASE` the case directory, which is also the working directory.
//!
//! Output is normalized before comparison: the package version in `kashika <version>` and
//! the JSON `"tool"` field becomes `<tool>`, the
//! temporary, fixtures and case directories become `$TMP`, `$FIXTURES` and `$CASE`, and
//! `(os error N)` becomes `(os error)`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use libtest_mimic::{Arguments, Failed, Trial};

const EXPECT: [&str; 3] = ["expect.stdout", "expect.stderr", "expect.code"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cli-snapshots")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn bless() -> bool {
    std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1")
}

fn main() {
    let args = Arguments::from_args();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root())
        .expect("tests/cli-snapshots exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("argv").is_file())
        .collect();
    dirs.sort();
    let trials = dirs
        .into_iter()
        .map(|dir| {
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            let unix_only = std::fs::read_to_string(dir.join("setup"))
                .is_ok_and(|s| s.lines().any(|l| l.starts_with("chmod ")));
            Trial::test(name.clone(), move || run_case(&name, &dir))
                .with_ignored_flag(unix_only && !cfg!(unix))
        })
        .collect();
    libtest_mimic::run(&args, trials).exit();
}

struct Vars {
    case: PathBuf,
    tmp: PathBuf,
    fixtures: PathBuf,
}

impl Vars {
    fn expand(&self, s: &str) -> String {
        s.replace("$FIXTURES", &self.fixtures.to_string_lossy())
            .replace("$TMP", &self.tmp.to_string_lossy())
            .replace("$CASE", &self.case.to_string_lossy())
    }

    fn normalize(&self, s: &str) -> String {
        let mut s = s.to_string();
        for (dir, var) in [
            (&self.tmp, "$TMP"),
            (&self.fixtures, "$FIXTURES"),
            (&self.case, "$CASE"),
        ] {
            let canonical = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            for form in [canonical, dir.clone()] {
                s = s.replace(&*form.to_string_lossy(), var);
            }
        }
        let v = env!("CARGO_PKG_VERSION");
        s = s
            .replace(&format!("kashika {v}"), "kashika <tool>")
            .replace(&format!("\"tool\":\"{v}\""), "\"tool\":\"<tool>\"");
        os_errors(&s)
    }
}

/// `s` with every `(os error N)` written as `(os error)`, since the numbers vary by platform.
fn os_errors(s: &str) -> String {
    const PAT: &str = "(os error ";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(PAT) {
        out.push_str(&rest[..i]);
        let after = &rest[i + PAT.len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with(')') {
            out.push_str("(os error)");
            rest = &after[digits + 1..];
        } else {
            out.push_str(PAT);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn setup(vars: &Vars, dir: &Path) -> Result<bool, String> {
    let Ok(script) = std::fs::read_to_string(dir.join("setup")) else {
        return Ok(true);
    };
    for line in script.lines().filter(|l| !l.trim().is_empty()) {
        let words: Vec<String> = line.split_whitespace().map(|w| vars.expand(w)).collect();
        match words.as_slice() {
            [op, from, to] if op == "copy" => {
                std::fs::copy(dir.join(from), dir.join(to))
                    .map_err(|e| format!("setup `{line}`: {e}"))?;
            }
            [op, path] if op == "mkdir" => {
                std::fs::create_dir_all(dir.join(path))
                    .map_err(|e| format!("setup `{line}`: {e}"))?;
            }
            [op, mode, path] if op == "chmod" => {
                let mode =
                    u32::from_str_radix(mode, 8).map_err(|e| format!("setup `{line}`: {e}"))?;
                if !chmod(&dir.join(path), mode).map_err(|e| format!("setup `{line}`: {e}"))? {
                    return Ok(false);
                }
            }
            _ => return Err(format!("setup: unknown line `{line}`")),
        }
    }
    Ok(true)
}

/// Sets `path`'s mode. False when a mode without read permission still lets this process
/// read the file (as root does), so the case cannot observe the denial.
#[cfg(unix)]
fn chmod(path: &Path, mode: u32) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(mode & 0o444 != 0 || std::fs::File::open(path).is_err())
}

#[cfg(not(unix))]
fn chmod(_: &Path, _: u32) -> std::io::Result<bool> {
    Ok(false)
}

fn run_case(name: &str, dir: &Path) -> Result<(), Failed> {
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let vars = Vars {
        case: dir.to_path_buf(),
        tmp: tmp.path().to_path_buf(),
        fixtures: fixtures(),
    };
    if !setup(&vars, dir)? {
        eprintln!("{name}: skipped, this process can read a file with no read permission");
        return Ok(());
    }
    let argv = std::fs::read_to_string(dir.join("argv")).map_err(|e| e.to_string())?;
    let args: Vec<String> = argv.lines().map(|a| vars.expand(a)).collect();
    let stdin = std::fs::read(dir.join("stdin")).unwrap_or_default();

    let mut child = Command::new(env!("CARGO_BIN_EXE_kashika"))
        .current_dir(dir)
        .env_remove("KASHIKA_ICON_PATH")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut pipe = child.stdin.take().expect("stdin is piped");
    let writer = std::thread::spawn(move || {
        use std::io::Write;
        let _ = pipe.write_all(&stdin);
    });
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = writer.join();

    let code = out
        .status
        .code()
        .map_or_else(|| "signal".to_string(), |c| c.to_string());
    let actual = [
        vars.normalize(&String::from_utf8_lossy(&out.stdout)),
        vars.normalize(&String::from_utf8_lossy(&out.stderr)),
        format!("{code}\n"),
    ];

    if bless() {
        for (file, text) in EXPECT.iter().zip(&actual) {
            std::fs::write(dir.join(file), text).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let mut problems = Vec::new();
    for (file, text) in EXPECT.iter().zip(&actual) {
        match std::fs::read_to_string(dir.join(file)) {
            Err(e) => problems.push(format!("{file}: {e}")),
            Ok(expected) if expected == *text => {}
            Ok(expected) => problems.push(format!(
                "{file} differs:\n{}",
                similar::TextDiff::from_lines(&expected, text)
                    .unified_diff()
                    .context_radius(3)
                    .header("expected", "actual")
            )),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{name}\n{}\nif the change is intended, run: KASHIKA_BLESS=1 cargo test -p kashika --test snapshots -- {name}",
            problems.join("\n")
        )
        .into())
    }
}
