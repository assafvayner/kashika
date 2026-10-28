use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

#[test]
fn prints_ranked_ready_to_paste_candidates() {
    kashika()
        .args(["route", &fixture("route.kash"), "a.top", "b.top"])
        .assert()
        .code(0)
        .stdout(
            "#1  crossings=0 near=0 segments=3 length=444px  via corridor y 0..34\n    line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow\n",
        );
}

#[test]
fn style_argument_selects_arc() {
    kashika()
        .args([
            "route",
            &fixture("route.kash"),
            "a.top",
            "b.top",
            "arc",
            "--radius",
            "6",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("end=arrow radius=6"));
}

#[test]
fn json_output() {
    let out = kashika()
        .args([
            "route",
            &fixture("route.kash"),
            "a.top",
            "b.top",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.route/1");
    assert_eq!(v["style"], "corner");
    assert_eq!(v["from"]["side"], "top");
    assert_eq!(v["candidates"][0]["segments"], 3);
    assert!(
        v["candidates"][0]["statement"]
            .as_str()
            .unwrap()
            .starts_with("line a_b ")
    );
}

#[test]
fn reroutes_with_line_flag() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("r.kash");
    let src = std::fs::read_to_string(fixture("route.kash")).unwrap();
    std::fs::write(
        &f,
        format!("{src}line ab a.top -> y 28 -> hv b.top end=diamond\n"),
    )
    .unwrap();
    kashika()
        .args(["route", f.to_str().unwrap(), "--line", "ab"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "line ab a.top -> y wall.tl+(0,-12) -> hv b.top end=diamond",
        ));
}

#[test]
fn no_path_exits_one() {
    kashika()
        .args(["route", "-", "a.right", "b.left"])
        .write_stdin(
            "diagram \"b\" size (300,200)\n\
box a at (20,80) size (100,40)\n\
box c at (122,20) size (40,160)\n\
box b at (220,80) size (60,40)\n",
        )
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "no candidate path avoids every box with at most 4 elbows",
        ));
}

#[test]
fn detours_when_no_simple_path_is_clear() {
    kashika()
        .args([
            "route",
            &fixture("route.kash"),
            "a.right",
            "b.left",
            "--max",
            "1",
        ])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("segments=5"))
        .stdout(predicate::str::contains("line a_b a.right -> x "));
}

#[test]
fn unknown_anchor_exits_two() {
    kashika()
        .args(["route", &fixture("route.kash"), "a.tpo", "b.top"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("error[unknown-anchor]"));
}

#[test]
fn one_point_is_a_usage_error() {
    kashika()
        .args(["route", &fixture("route.kash"), "a.top"])
        .assert()
        .code(3);
}

#[test]
fn unknown_line_is_a_usage_error() {
    kashika()
        .args(["route", &fixture("route.kash"), "--line", "nope"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("no line `nope`"));
}
