use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

#[test]
fn render_writes_png() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.png");
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("output: written"));
    assert_eq!(&std::fs::read(&out).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn force_draws_red_markers_at_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    kashika()
        .args([
            "render",
            &fixture("typo.kash"),
            "-o",
            out.to_str().unwrap(),
            "--force",
        ])
        .assert()
        .code(2);
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("<g id=\"kashika-overlay\">"));
    assert!(svg.contains("error[unknown-anchor]"));
}

#[test]
fn clean_render_has_no_overlay_and_debug_adds_one() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.svg");
    let debug = dir.path().join("debug.svg");
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            plain.to_str().unwrap(),
        ])
        .assert()
        .code(0);
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            debug.to_str().unwrap(),
            "--debug",
        ])
        .assert()
        .code(0);
    assert!(!std::fs::read_to_string(&plain)
        .unwrap()
        .contains("kashika-overlay"));
    assert!(std::fs::read_to_string(&debug)
        .unwrap()
        .contains("kashika-overlay"));
}

#[test]
fn debug_marks_lint_warnings_at_their_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("debug.svg");
    kashika()
        .args([
            "render",
            &fixture("crossing.kash"),
            "-o",
            out.to_str().unwrap(),
            "--debug",
        ])
        .assert()
        .code(1);
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(svg.contains("<circle cx=\"200\" cy=\"150\" r=\"6\" fill=\"none\" stroke=\"#ea580c\""));
    assert!(svg.contains("aria-label=\"warn[cross]\""));
    assert!(
        !svg.contains("<circle cx=\"240\" cy=\"150\""),
        "suppressed findings are not marked"
    );
}
