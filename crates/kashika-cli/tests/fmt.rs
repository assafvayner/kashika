#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const MESSY: &str = "diagram  \"d\" size (200, 100)\nbox a at ( 10,10 ) size (40, 20)   // a box\n\n\n\nbox b at a.right+( 40 ,0) size (40, 20) pin=left\n";
const CLEAN: &str = "diagram \"d\" size (200, 100)\nbox a at (10, 10) size (40, 20)  // a box\n\nbox b at a.right+(40, 0) size (40, 20) pin=left\n";

#[test]
fn rewrites_file_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    std::fs::write(&f, MESSY).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(0)
        .stdout("");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CLEAN);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn check_reports_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    std::fs::write(&f, MESSY).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap(), "--check"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("not formatted"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), MESSY);
    std::fs::write(&f, CLEAN).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap(), "--check"])
        .assert()
        .code(0);
}

#[test]
fn format_is_an_alias_of_fmt() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    std::fs::write(&f, MESSY).unwrap();
    kashika()
        .args(["format", f.to_str().unwrap()])
        .assert()
        .code(0);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CLEAN);
}

#[test]
fn keeps_crlf_line_endings() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    let clean = CLEAN.replace('\n', "\r\n");
    std::fs::write(&f, &clean).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap(), "--check"])
        .assert()
        .code(0);
    std::fs::write(&f, MESSY.replace('\n', "\r\n")).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(0);
    let out = std::fs::read_to_string(&f).unwrap();
    assert_eq!(out, clean);
    assert!(!out.replace("\r\n", "").contains('\n'), "{out:?}");
}

#[test]
fn parse_errors_exit_two_and_leave_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("bad.kash");
    std::fs::write(&f, "box a at\n").unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("error[parse]"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "box a at\n");
}

#[test]
fn stdin_name_names_stdin_in_reports() {
    kashika()
        .args(["fmt", "-", "--stdin-name", "draft.kash"])
        .write_stdin("box a at\n")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("draft.kash:1:"))
        .stdout(predicate::str::contains("<stdin>").not());
    kashika()
        .args(["fmt", "-", "--check", "--stdin-name", "draft.kash"])
        .write_stdin("diagram \"d\" size (100, 100)\nbox  a at (0, 0) size (10, 10)\n")
        .assert()
        .code(1)
        .stdout("draft.kash: not formatted\n");
}

#[test]
fn missing_file_is_a_tool_error() {
    kashika().args(["fmt", "nope.kash"]).assert().code(3);
}

/// Every .kash in the repo formats idempotently and renders the same before and after.
#[test]
fn corpus_round_trips() {
    let mut files = Vec::new();
    let mut dirs = vec![root().join("crates/kashika-cli/tests/fixtures")];
    for e in std::fs::read_dir(root().join("examples")).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            dirs.push(p);
        }
    }
    for dir in dirs {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "kash") {
                files.push(p);
            }
        }
    }
    assert!(!files.is_empty());
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let Ok(once) = kashika_syntax::format(&src) else {
            continue;
        };
        assert_eq!(
            kashika_syntax::format(&once).unwrap(),
            once,
            "{}",
            f.display()
        );
        let (a, da) = kashika_model::compile(&src);
        let (b, db) = kashika_model::compile(&once);
        assert_eq!(
            kashika_render::render_svg(&a),
            kashika_render::render_svg(&b),
            "{}",
            f.display()
        );
        assert_eq!(codes(&da), codes(&db), "{}", f.display());
    }
    for f in walk_kash(&root().join("tests/golden"), "input.kash") {
        let src = std::fs::read_to_string(&f).unwrap();
        let Ok(once) = kashika_syntax::format(&src) else {
            continue;
        };
        let opts = kashika_model::Options {
            path: Some(f.clone()),
            ..Default::default()
        };
        let checked = |src: &str| {
            let (scene, mut diags) = kashika_model::compile_with(src, &opts);
            diags.extend(kashika_check::check(&scene));
            kashika_diag::sort(&mut diags);
            codes(&diags)
        };
        assert_eq!(checked(&src), checked(&once), "{}", f.display());
    }
}

fn codes(d: &[kashika_diag::Diagnostic]) -> Vec<(&'static str, String)> {
    d.iter().map(|d| (d.code, d.message.clone())).collect()
}

/// Every file under `dir`, recursively, named `name` or, for `*.kash`, with that extension.
fn walk_kash(dir: &std::path::Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| n == name)
                || (name == "*.kash" && p.extension().is_some_and(|x| x == "kash"))
            {
                out.push(p);
            }
        }
    }
    out.sort();
    assert!(!out.is_empty(), "no {name} under {}", dir.display());
    out
}

/// The source text of every token and comment in order, without carriage returns.
fn token_texts(src: &str) -> Vec<String> {
    use kashika_syntax::Tok;
    let (toks, comments, _) = kashika_syntax::lex_with_comments(src);
    let text = kashika_syntax::SourceText::new(src);
    let mut items: Vec<(u32, u32, String)> = toks
        .iter()
        .filter(|t| !matches!(t.tok, Tok::Newline | Tok::Eof))
        .map(|t| (t.span.line, t.span.col, text.span(t.span)))
        .chain(
            comments
                .into_iter()
                .map(|c| (c.span.line, c.span.col, c.text)),
        )
        .collect();
    items.sort_by_key(|(line, col, _)| (*line, *col));
    items
        .into_iter()
        .map(|(_, _, s)| s.replace('\r', ""))
        .collect()
}

#[test]
fn fmt_keeps_token_and_comment_text_of_examples() {
    for f in walk_kash(&root().join("examples"), "*.kash") {
        let src = std::fs::read_to_string(&f).unwrap();
        let once =
            kashika_syntax::format(&src).unwrap_or_else(|d| panic!("{}: {d:#?}", f.display()));
        assert_eq!(token_texts(&src), token_texts(&once), "{}", f.display());
    }
}

#[test]
fn pin_with_position_round_trips() {
    let src = "diagram \"d\" size (200, 100)\nbox a at (10, 10) size (40, 20) pin=left@0.25\n";
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    std::fs::write(&f, src).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap(), "--check"])
        .assert()
        .code(0);
    std::fs::write(&f, src.replace("pin=left@0.25", "pin=left @ 0.25")).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(0);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
}

#[test]
fn color_names_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    let src = "diagram \"d\" size (200, 100) background=Ivory\nbox a at (10, 10) size (40, 20) fill=SteelBlue color=white\n";
    std::fs::write(&f, src).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap(), "--check"])
        .assert()
        .code(0);
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(0);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), src);
}

#[cfg(unix)]
#[test]
fn writes_through_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.kash");
    let link = dir.path().join("link.kash");
    std::fs::write(&real, MESSY).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    kashika()
        .args(["fmt", link.to_str().unwrap()])
        .assert()
        .code(0);
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_to_string(&real).unwrap(), CLEAN);
}

#[cfg(unix)]
#[test]
fn preserves_file_mode() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("d.kash");
    std::fs::write(&f, MESSY).unwrap();
    std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o640)).unwrap();
    kashika()
        .args(["fmt", f.to_str().unwrap()])
        .assert()
        .code(0);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), CLEAN);
    assert_eq!(
        std::fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[cfg(unix)]
#[test]
fn render_writes_through_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("d.kash");
    let real = dir.path().join("real.svg");
    let link = dir.path().join("link.svg");
    std::fs::write(&src, CLEAN).unwrap();
    std::fs::write(&real, "old").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    kashika()
        .args([
            "render",
            src.to_str().unwrap(),
            "-o",
            link.to_str().unwrap(),
        ])
        .assert()
        .code(0);
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(std::fs::read_to_string(&real).unwrap().starts_with("<svg"));
}
