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
            "#1  crossings=0 transits=0 near=0 segments=3 length=444px  via corridor y 0..34\n    line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow\n",
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
fn rerouting_an_arrow_keeps_the_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("r.kash");
    let src = std::fs::read_to_string(fixture("route.kash")).unwrap();
    std::fs::write(&f, format!("{src}arrow ab a.top -> y 28 -> hv b.top\n")).unwrap();
    kashika()
        .args(["route", f.to_str().unwrap(), "--line", "ab"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "arrow ab a.top -> y wall.tl+(0,-12) -> hv b.top\n",
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
        .stdout(
            "#1  crossings=0 transits=0 near=0 segments=5 length=384px  via corridor y 0..34\n    line a_b a.right -> x a.tr+(12,0) -> y wall.tl+(0,-12) -> x b.tl+(-12,0) -> vh b.left end=arrow\n",
        );
}

#[test]
fn ranks_by_near_misses_then_length() {
    kashika()
        .args([
            "route",
            &fixture("route-tiers.kash"),
            "a.bottom",
            "b.bottom",
            "--max",
            "50",
        ])
        .assert()
        .code(0)
        .stdout(
            "#1  crossings=0 transits=0 near=0 segments=3 length=216px  via corridor y 66..70\n    line a_b a.bottom -> y 68 -> hv b.bottom end=arrow\n\
             #2  crossings=0 transits=0 near=0 segments=3 length=336px  via corridor y 122..200\n    line a_b a.bottom -> y c.bl+(0,12) -> hv b.bottom end=arrow\n\
             #3  crossings=0 transits=0 near=1 segments=3 length=224px\n    line a_b a.bottom -> y a.bl+(0,12) -> hv b.bottom end=arrow\n\
             #4  crossings=0 transits=0 near=2 segments=3 length=208px\n    line a_b a.bottom -> y c.tl+(0,-12) -> hv b.bottom end=arrow\n",
        );
}

#[test]
fn crossings_outrank_near_misses() {
    kashika()
        .args([
            "route",
            &fixture("route-tiers-blocked.kash"),
            "a.bottom",
            "b.bottom",
            "--max",
            "50",
        ])
        .assert()
        .code(0)
        .stdout(
            "#1  crossings=0 transits=0 near=2 segments=3 length=208px\n    line a_b a.bottom -> y c.tl+(0,-12) -> hv b.bottom end=arrow\n\
             #2  crossings=2 (p at (40,70), p at (240,70)) transits=0 near=0 segments=3 length=336px  via corridor y 122..200\n    line a_b a.bottom -> y c.bl+(0,12) -> hv b.bottom end=arrow\n\
             #3  crossings=2 (p at (40,70), p at (240,70)) transits=0 near=2 segments=3 length=224px\n    line a_b a.bottom -> y a.bl+(0,12) -> hv b.bottom end=arrow\n",
        );
}

/// Compares `actual` with `tests/snapshots/<name>`, or writes it with `KASHIKA_BLESS=1`.
fn snapshot(name: &str, actual: &str) {
    let path = format!("{}/tests/snapshots/{name}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("KASHIKA_BLESS").is_some() {
        std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; bless with KASHIKA_BLESS=1"));
    assert_eq!(
        actual, expected,
        "{name} changed; bless with KASHIKA_BLESS=1"
    );
}

#[test]
fn json_output_matches_the_snapshots() {
    for (snap, file, from, to) in [
        ("route-wall.json", "route.kash", "a.top", "b.top"),
        (
            "route-tiers-blocked.json",
            "route-tiers-blocked.kash",
            "a.bottom",
            "b.bottom",
        ),
    ] {
        let out = kashika()
            .current_dir(fixture(""))
            .args(["route", file, from, to, "--format", "json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{snap}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        let tool = format!("\"tool\":\"{}\"", env!("CARGO_PKG_VERSION"));
        assert_eq!(stdout.matches(&tool).count(), 1, "{stdout}");
        snapshot(snap, &stdout.replace(&tool, "\"tool\":\"<tool>\""));
    }
}

fn first_line(out: &std::process::Output) -> (Option<i32>, String) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    (
        out.status.code(),
        stdout.lines().next().unwrap_or_default().to_string(),
    )
}

#[test]
fn flags_reach_the_route() {
    let wall_top =
        "#1  crossings=0 transits=0 near=0 segments=3 length=444px  via corridor y 0..34";
    let run = |args: &[&str]| first_line(&kashika().args(args).output().unwrap());
    let route = fixture("route.kash");
    assert_eq!(
        run(&["route", &route, "a.top", "b.top", "arc", "--radius", "0"]),
        (Some(0), wall_top.to_string()),
        "`radius=0` corners still count as segments"
    );
    assert_eq!(
        kashika()
            .args(["route", &route, "a.top", "b.top", "arc", "--radius", "0"])
            .output()
            .unwrap()
            .stdout,
        format!(
            "{wall_top}\n    line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow radius=0\n"
        )
        .into_bytes()
    );
    kashika()
        .args(["route", &route, "a.right", "b.left", "--max", "1"])
        .assert()
        .code(0)
        .stdout(predicate::function(|s: &str| s.lines().count() == 2));
    let piped = kashika()
        .args(["route", "-", "a.top", "b.top", "--stdin-name", "w.kash"])
        .write_stdin(std::fs::read_to_string(&route).unwrap())
        .output()
        .unwrap();
    assert_eq!(first_line(&piped), (Some(0), wall_top.to_string()));
    let piped = kashika()
        .args(["route", "-", "a.rigth", "b.left", "--stdin-name", "w.kash"])
        .write_stdin(std::fs::read_to_string(&route).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        first_line(&piped),
        (
            Some(2),
            "w.kash: error[unknown-anchor]: unknown anchor `rigth` on `a`".to_string()
        )
    );

    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("shared")).unwrap();
    std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
    let icon = tmp.path().join("shared/x.svg");
    std::fs::write(
        &icon,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><rect width="20" height="20"/></svg>"#,
    )
    .unwrap();
    let src = std::fs::read_to_string(&route).unwrap().replace(
        "box b \"B\" at (320,100) size (100,40)",
        "box b \"B\" at (320,100) size (100,40) icon=\"../shared/x.svg\"",
    );
    let file = tmp.path().join("proj/a.kash");
    std::fs::write(&file, src).unwrap();
    let file = file.to_str().unwrap();
    let (code, first) = run(&["route", file, "a.top", "b.top"]);
    assert_eq!(code, Some(2));
    assert!(first.contains("error[file]"), "{first}");
    assert_eq!(
        run(&["route", file, "a.top", "b.top", "--allow-path"]),
        (Some(0), wall_top.to_string())
    );
    assert_eq!(
        run(&[
            "route",
            file,
            "a.top",
            "b.top",
            "--include",
            icon.to_str().unwrap()
        ]),
        (Some(0), wall_top.to_string())
    );
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
fn input_with_errors_is_reported_like_check() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("e.kash");
    std::fs::write(
        &f,
        "diagram \"e\" size (200,100)\nbox a at (10,10) size (40,20)\nbox b at (100,10) size (40,20)\nline a.rigth -> b.left\nline (30,0) -> v 100\n",
    )
    .unwrap();
    let f = f.to_str().unwrap();
    let route = kashika()
        .args(["route", f, "a.right", "b.left"])
        .output()
        .unwrap();
    assert_eq!(route.status.code(), Some(2));
    let check = kashika().args(["check", f]).output().unwrap();
    assert_eq!(check.status.code(), Some(2));
    let route = String::from_utf8(route.stdout).unwrap();
    assert!(route.contains("error[unknown-anchor]"), "{route}");
    let check = String::from_utf8(check.stdout).unwrap();
    let (body, metrics) = check.trim_end().rsplit_once(" · ").unwrap();
    assert!(metrics.starts_with("crossings="), "{metrics}");
    assert!(body.ends_with("1 error, 1 warning"), "{body}");
    assert_eq!(route, format!("{body}\n"));

    let out = kashika()
        .args(["route", f, "a.right", "b.left", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.route/1");
    assert_eq!(v["input"]["name"], f);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 2);
    assert_eq!(v["diagnostics"][0]["code"], "unknown-anchor");
    assert_eq!(v["diagnostics"][1]["code"], "through");
    assert!(v.get("candidates").is_none());
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

#[test]
fn hints_at_line_with_same_endpoints() {
    let ex = format!(
        "{}/../../examples/upload/upload.kash",
        env!("CARGO_MANIFEST_DIR")
    );
    kashika()
        .args(["route", &ex, "api1.bottom", "queue.left"])
        .assert()
        .stdout(predicate::str::contains(
            "hint: line `api1_queue` already connects these; pass --line api1_queue",
        ));
    kashika()
        .args(["route", &ex, "--line", "api1_queue"])
        .assert()
        .stdout(predicate::str::contains("hint:").not());
}

#[test]
fn notes_allow_entries_a_suggestion_no_longer_needs() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("r.kash");
    std::fs::write(
        &f,
        "diagram \"r\" size (460,240)\n\
box a \"A\" at (20,100) size (100,40)\n\
box b \"B\" at (320,100) size (100,40)\n\
line l_feat (220,60) -> (220,180)\n\
line ab a.right -> b.left end=arrow allow=cross:l_feat reason=\"shared bus\"\n",
    )
    .unwrap();
    let f = f.to_str().unwrap();
    kashika()
        .args(["route", f, "--line", "ab", "--max", "1"])
        .assert()
        .code(0)
        .stdout(
            "#1  crossings=0 transits=0 near=0 segments=5 length=386px  via corridor y 0..54\n    line ab a.right -> x a.tr+(12,0) -> y 27 -> x b.tl+(-12,0) -> vh b.left end=arrow\n    note: dropped allow=cross:l_feat (no longer needed)\n",
        );
    let out = kashika()
        .args(["route", f, "--line", "ab", "--max", "1", "--format", "json"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["candidates"][0]["dropped_allow"],
        serde_json::json!(["cross:l_feat"])
    );
}
