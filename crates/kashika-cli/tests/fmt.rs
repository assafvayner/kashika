use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const MESSY: &str = "diagram  \"d\" size (200,100)\nbox a at ( 10,10 ) size (40,20)   // a box\n\n\n\nbox b at a.right+( 40 ,0) size (40,20) pin=left\n";
const CLEAN: &str = "diagram \"d\" size (200,100)\nbox a at (10,10) size (40,20)  // a box\n\nbox b at a.right+(40,0) size (40,20) pin=left\n";

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
fn missing_file_is_a_tool_error() {
    kashika().args(["fmt", "nope.kash"]).assert().code(3);
}

/// Every .kash in the repo formats idempotently and renders the same before and after.
#[test]
fn corpus_round_trips() {
    let mut files = Vec::new();
    for dir in ["examples", "crates/kashika-cli/tests/fixtures"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
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
        let codes = |d: &[kashika_diag::Diagnostic]| {
            d.iter()
                .map(|d| (d.code, d.message.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(codes(&da), codes(&db), "{}", f.display());
    }
}
