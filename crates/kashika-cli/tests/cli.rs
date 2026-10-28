use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

#[test]
fn check_clean_file_exits_zero() {
    kashika()
        .args(["check", &fixture("basic.kash")])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("0 errors, 0 warnings"));
}

#[test]
fn check_reports_typo_with_source_and_suggestion() {
    kashika()
        .args(["check", &fixture("typo.kash")])
        .assert()
        .code(2)
        .stdout(
            predicate::str::contains(
                "typo.kash:4:10: error[unknown-anchor]: unknown anchor `rigth` on `api`",
            )
            .and(predicate::str::contains(
                "    4 | line api.rigth -> db.left end=arrow",
            ))
            .and(predicate::str::contains("= help: did you mean `right`?"))
            .and(predicate::str::contains("1 error, 0 warnings")),
        );
}

#[test]
fn json_envelope() {
    let out = kashika()
        .args(["check", &fixture("typo.kash"), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.check/1");
    assert_eq!(v["counts"]["error"], 1);
    assert_eq!(v["truncated"], false);
    assert_eq!(v["output"], serde_json::Value::Null);
    assert_eq!(v["input"]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(v["diagnostics"][0]["code"], "unknown-anchor");
    assert_eq!(v["diagnostics"][0]["span"]["line"], 4);
}

#[test]
fn render_writes_svg() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
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
    assert!(std::fs::read_to_string(&out).unwrap().starts_with("<svg"));
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no temp files left behind"
    );
}

#[test]
fn render_does_not_write_on_error_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    kashika()
        .args(["render", &fixture("typo.kash"), "-o", out.to_str().unwrap()])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("output: not written"));
    assert!(!out.exists());
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
    assert!(out.exists());
}

#[test]
fn stdin_input_uses_stdin_name() {
    let src = std::fs::read_to_string(fixture("typo.kash")).unwrap();
    kashika()
        .args(["check", "-", "--stdin-name", "snippet.kash"])
        .write_stdin(src)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("snippet.kash:4:"));
}

#[test]
fn fail_on_controls_exit_codes() {
    kashika()
        .args(["check", &fixture("typo.kash"), "--fail-on", "never"])
        .assert()
        .code(0);
}

#[test]
fn non_svg_output_is_a_tool_error() {
    kashika()
        .args(["render", &fixture("basic.kash"), "-o", "x.png"])
        .assert()
        .code(3);
}

#[test]
fn usage_errors_exit_three_and_help_exits_zero() {
    kashika().args(["check"]).assert().code(3);
    kashika().args(["--help"]).assert().code(0);
}

#[test]
fn missing_file_is_a_tool_error() {
    kashika()
        .args(["check", "nope.kash"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("cannot read nope.kash"));
}
