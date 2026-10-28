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
fn unknown_output_extension_is_a_tool_error() {
    kashika()
        .args(["render", &fixture("basic.kash"), "-o", "x.pdf"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "output must end in .svg, .png, .jpg, .jpeg or .webp",
        ));
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

#[test]
fn bundled_examples_check_clean() {
    for name in ["upload", "shapes"] {
        let example = format!(
            "{}/../../examples/{name}/{name}.kash",
            env!("CARGO_MANIFEST_DIR")
        );
        kashika()
            .args(["check", &example])
            .assert()
            .code(0)
            .stdout(predicate::str::contains("0 errors, 0 warnings"));
    }
}

#[test]
fn failed_write_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    std::fs::create_dir(&out).unwrap();
    kashika()
        .args([
            "render",
            &fixture("basic.kash"),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(3);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec!["out.svg"], "no temp files left behind");
}

#[test]
fn huge_coordinates_fail_cleanly_even_when_forced() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.svg");
    let big = "1".repeat(300);
    let src = format!(
        "diagram \"x\" size ({big},100)\nbox a at (0,0) size ({big},10)\nline l (0,0) -> curve via ({big},0) (0,{big}) (10,10) end=arrow\n"
    );
    kashika()
        .args(["render", "-", "-o", out.to_str().unwrap(), "--force"])
        .write_stdin(src)
        .timeout(std::time::Duration::from_secs(20))
        .assert()
        .code(2);
    let svg = std::fs::read_to_string(&out).unwrap();
    assert!(!svg.contains("inf") && !svg.contains("NaN"), "{svg}");
}

#[test]
fn measure_prints_width_and_height() {
    kashika()
        .args(["measure", "API"])
        .assert()
        .code(0)
        .stdout("width=22.36 height=17.5 lines=1\n");
    kashika()
        .args(["measure", "API", "--size", "28", "--weight", "bold"])
        .assert()
        .code(0)
        .stdout("width=46.91 height=35 lines=1\n");
}

#[test]
fn measure_applies_markup_and_newlines() {
    kashika()
        .args(["measure", r"`API`\nPostgres"])
        .assert()
        .code(0)
        .stdout("width=58.71 height=35 lines=2\n");
}

#[test]
fn measure_json() {
    let out = kashika()
        .args(["measure", "SQL", "--size", "12", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.measure/1");
    assert_eq!(v["width"], 23.66);
    assert_eq!(v["height"], 15.0);
    assert_eq!(v["lines"], 1);
}

#[test]
fn measure_reports_unclosed_markup_and_bad_sizes() {
    kashika()
        .args(["measure", "**API"])
        .assert()
        .code(2)
        .stdout("error[parse]: unclosed `**` in text\n");
    kashika()
        .args(["measure", "API", "--size", "0"])
        .assert()
        .code(3);
}

#[test]
fn check_reports_label_overflow() {
    kashika()
        .args(["check", &fixture("overflow.kash")])
        .assert()
        .code(2)
        .stdout(
            predicate::str::contains(
                "overflow.kash:2:1: error[label-overflow]: its text needs 91.64x17.5px but the text area of `api` is 64x14px (too small: 27.64px narrower, 3.5px shorter)",
            )
            .and(predicate::str::contains(
                "= help: make `api` at least size (108,34), or shorten the text",
            )),
        );
}

#[test]
fn text_fixture_checks_clean_and_renders_deterministically() {
    kashika()
        .args(["check", &fixture("text.kash")])
        .assert()
        .code(0);
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.svg");
    let b = dir.path().join("b.svg");
    for out in [&a, &b] {
        kashika()
            .args(["render", &fixture("text.kash"), "-o", out.to_str().unwrap()])
            .assert()
            .code(0);
    }
    let svg = std::fs::read_to_string(&a).unwrap();
    assert_eq!(svg, std::fs::read_to_string(&b).unwrap());
    assert!(svg.contains(r#"aria-label="Worker""#));
    assert!(svg.contains(r#"aria-label="• retries charge()""#));
    assert!(!svg.contains("<text"));
}

#[test]
fn warnings_suppression_and_metrics_in_text() {
    kashika()
        .args(["check", &fixture("crossing.kash")])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("crossing.kash:5:1: warn[cross]: v1 crosses ab at (200,150)")
                .and(predicate::str::contains(
                    "    5 | line v1 (200,20) -> v 260",
                ))
                .and(predicate::str::contains(
                    "    4 | line ab a.right -> b.left end=arrow",
                ))
                .and(predicate::str::contains(
                    "= help: if intentional: allow=cross:ab on v1",
                ))
                .and(predicate::str::contains(
                    "0 errors, 1 warning (1 suppressed) · crossings=2 wire=720px",
                ))
                .and(predicate::str::contains("v2 crosses").not()),
        );
}

#[test]
fn show_suppressed_lists_them_with_the_reason() {
    kashika()
        .args(["check", &fixture("crossing.kash"), "--show-suppressed"])
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("warn[cross]: v2 crosses ab at (240,150) (suppressed)")
                .and(predicate::str::contains("      = reason: bus")),
        );
}

#[test]
fn json_has_counts_metrics_elements_and_facts() {
    let out = kashika()
        .args(["check", &fixture("crossing.kash"), "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["counts"],
        serde_json::json!({"error": 0, "warn": 1, "suppressed": 1})
    );
    assert_eq!(
        v["metrics"],
        serde_json::json!({"crossings": 2, "wire_px": 720})
    );
    let diags = v["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0]["elements"][0]["id"], "v1");
    assert_eq!(diags[0]["elements"][0]["segment"], 1);
    assert_eq!(diags[0]["elements"][1]["id"], "ab");
    assert_eq!(diags[0]["facts"]["at"], serde_json::json!([200.0, 150.0]));
    assert_eq!(diags[0]["suggestions"], serde_json::json!([]));
}

#[test]
fn focus_keeps_only_matching_warnings() {
    kashika()
        .args(["check", &fixture("crossing.kash"), "--focus", "b"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("0 errors, 0 warnings"));
    kashika()
        .args(["check", &fixture("crossing.kash"), "--focus", "a,v1"])
        .assert()
        .code(1);
    kashika()
        .args(["check", &fixture("crossing.kash"), "--focus", "v3"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "unknown id `v3` in --focus; did you mean `v1`?",
        ));
}

#[test]
fn warnings_exit_one_unless_fail_on_relaxes_it() {
    for (fail_on, code) in [("warn", 1), ("error", 0), ("never", 0)] {
        kashika()
            .args(["check", &fixture("crossing.kash"), "--fail-on", fail_on])
            .assert()
            .code(code);
    }
}

#[test]
fn render_still_writes_with_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("x.svg");
    kashika()
        .args([
            "render",
            &fixture("crossing.kash"),
            "-o",
            out.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("output: written"));
    assert!(out.exists());
}

#[test]
fn inspect_prints_geometry_in_text() {
    kashika()
        .args(["inspect", &fixture("basic.kash")])
        .assert()
        .code(0)
        .stdout(
            predicate::str::contains("box api \"API\" (line 2)")
                .and(predicate::str::contains(
                    "  anchors top (100,40) bottom (100,100) left (40,70) right (160,70)",
                ))
                .and(predicate::str::contains(
                    "line api_db (line 4) length 160px",
                ))
                .and(predicate::str::contains("  segment 1 to (4:"))
                .and(predicate::str::contains("  label@0.5 \"SQL\" at (240,70)"))
                .and(predicate::str::contains("placement\n  db -> api (at)"))
                .and(predicate::str::contains("0 errors, 0 warnings")),
        );
}

#[test]
fn inspect_json_with_ids_ports_and_corridors() {
    let out = kashika()
        .args([
            "inspect",
            &fixture("basic.kash"),
            "db",
            "--ports",
            "--corridors",
            "0,0,480,240",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.inspect/1");
    assert_eq!(v["shapes"].as_array().unwrap().len(), 1);
    assert_eq!(
        v["shapes"][0]["bbox"],
        serde_json::json!([[320.0, 40.0], [440.0, 100.0]])
    );
    assert_eq!(
        v["shapes"][0]["anchors"]["left"],
        serde_json::json!([320.0, 70.0])
    );
    assert_eq!(
        v["shapes"][0]["local_anchors"]["left"],
        serde_json::json!([320.0, 70.0])
    );
    assert_eq!(
        v["ports"],
        serde_json::json!([{"shape": "db", "side": "left", "t": 0.5, "at": [320.0, 70.0], "line": "api_db", "end": "end"}])
    );
    assert_eq!(
        v["corridors"]["horizontal"],
        serde_json::json!([[0.0, 34.0], [106.0, 240.0]])
    );
    assert_eq!(
        v["corridors"]["vertical"],
        serde_json::json!([[0.0, 34.0], [446.0, 480.0]])
    );
    assert_eq!(
        v["placement"],
        serde_json::json!([{"from": "db", "to": "api", "kind": "at"}])
    );
}

#[test]
fn inspect_works_on_files_with_errors() {
    kashika()
        .args(["inspect", &fixture("typo.kash")])
        .assert()
        .code(0)
        .stdout(
            predicate::str::contains("box api \"API\" (line 2)")
                .and(predicate::str::contains("1 error, 0 warnings")),
        );
}

#[test]
fn inspect_rejects_unknown_ids_and_bad_corridors() {
    kashika()
        .args(["inspect", &fixture("basic.kash"), "nope"])
        .assert()
        .code(3);
    kashika()
        .args(["inspect", &fixture("basic.kash"), "--corridors", "1,2,3"])
        .assert()
        .code(3);
}

#[test]
fn seeded_mistakes_trigger_their_lints() {
    let out = kashika()
        .args([
            "check",
            &fixture("mistakes.kash"),
            "--format",
            "json",
            "--max",
            "200",
        ])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let codes: std::collections::BTreeSet<&str> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    for code in [
        "cross",
        "near",
        "overlap",
        "spill",
        "diagonal-tail",
        "port-crowding",
        "low-contrast",
        "out-of-canvas",
        "off-grid",
    ] {
        assert!(codes.contains(code), "{code} missing from {codes:?}");
    }
}

#[test]
fn library_files_check_on_their_own() {
    let dir = tempfile::tempdir().unwrap();
    let lib = dir.path().join("lib.kash");
    std::fs::write(&lib, "style s { fill=#fff }\nshape t from \"t.svg\"\n").unwrap();
    let lib = lib.to_str().unwrap();
    kashika().args(["check", lib]).assert().code(2).stdout(
        predicate::str::contains("lib.kash:2:14: error[file]: file `t.svg` not found")
            .and(predicate::str::contains("missing `diagram`").not())
            .and(predicate::str::contains("1 error, 0 warnings\n")),
    );
    std::fs::write(dir.path().join("lib.kash"), "style s { fill=#fff }\n").unwrap();
    kashika().args(["check", lib]).assert().code(0);
    kashika()
        .args(["render", lib, "-o"])
        .arg(dir.path().join("out.svg"))
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "is a library (no `diagram` statement); render a diagram that imports it",
        ));
    assert!(!dir.path().join("out.svg").exists());

    std::fs::write(
        dir.path().join("lib.kash"),
        "style s { fill=#fff }\nbox a at (0,0) size (1,1)\n",
    )
    .unwrap();
    kashika()
        .args(["check", lib])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("missing `diagram` statement"));
}

#[test]
fn repeated_errors_collapse_in_text_but_not_in_json() {
    let mut src = String::from("diagram \"t\" size (100,100)\n");
    for i in 0..10 {
        src.push_str(&format!("line l{i}: (0,{i}) -> h 10\n"));
    }
    let out = kashika()
        .args(["check", "-", "--stdin-name", "r.kash"])
        .write_stdin(src.clone())
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.matches("a line id takes no `:`").count(), 3, "{out}");
    assert!(
        out.contains(
            "      = help: remove the `:`: write `line l2 (0,2) -> h 10`\n... and 7 more like this\n10 errors, 0 warnings"
        ),
        "{out}"
    );
    kashika()
        .args(["check", "-", "--format", "json"])
        .write_stdin(src)
        .assert()
        .code(2)
        .stdout(predicate::function(|s: &str| {
            s.matches("a line id takes no `:`").count() == 10
        }));
}

#[test]
fn collapsed_errors_do_not_count_against_max() {
    let mut src = String::from("diagram \"t\" size (100,100)\n");
    for i in 0..6 {
        src.push_str(&format!("line l{i}: (0,{i}) -> h 10\n"));
    }
    src.push_str("box a at (50,50) size (10,10)\nbox a at (70,50) size (10,10)\n");
    kashika()
        .args(["check", "-", "--max", "3"])
        .write_stdin(src)
        .assert()
        .code(2)
        .stdout(
            predicate::str::contains("... and 3 more like this\n")
                .and(predicate::str::contains("error[duplicate-id]"))
                .and(predicate::str::contains("... and 1 more").not()),
        );
}

/// `n` boxes off a 100x100 canvas, one distinct `out-of-canvas` warning each, then a parse error.
fn warnings_then_error(n: usize) -> String {
    let mut src = String::from("diagram \"t\" size (100,100)\n");
    for i in 0..n {
        src.push_str(&format!("box b{i} at (200,{}) size (10,10)\n", i * 20));
    }
    src.push_str("line l: (0,0) -> h 10\n");
    src
}

#[test]
fn max_never_hides_errors_in_text() {
    let out = kashika()
        .args(["check", "-", "--stdin-name", "w.kash", "--max", "5"])
        .write_stdin(warnings_then_error(8))
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.matches(": warn[out-of-canvas]").count(), 4, "{out}");
    assert!(out.contains("w.kash:10:7: error[parse]"), "{out}");
    assert!(out.contains("... and 4 more\n1 error, 8 warnings"), "{out}");
}

#[test]
fn max_never_hides_errors_in_json() {
    let out = kashika()
        .args(["check", "-", "--format", "json", "--max", "3"])
        .write_stdin(warnings_then_error(8))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let diags = v["diagnostics"].as_array().unwrap();
    assert_eq!(diags.len(), 3);
    assert_eq!(v["truncated"], true);
    assert_eq!(v["counts"]["error"], 1);
    assert_eq!(diags.last().unwrap()["severity"], "error");
    assert_eq!(diags[0]["code"], "out-of-canvas");

    let out = kashika()
        .args(["check", "-", "--format", "json", "--max", "0"])
        .write_stdin(warnings_then_error(2))
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let codes: Vec<&str> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["parse"]);
    assert_eq!(v["truncated"], true);
}

#[test]
fn errors_print_even_when_they_alone_exceed_max() {
    let mut src = String::from("diagram \"t\" size (100,100)\n");
    for i in 0..4 {
        src.push_str(&format!(
            "box a{i} at (0,0) size (10,10)\nbox a{i} at (20,0) size (10,10)\n"
        ));
    }
    let out = kashika()
        .args(["check", "-", "--max", "1"])
        .write_stdin(src)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.matches("error[duplicate-id]").count(), 3, "{out}");
    assert!(
        out.contains("... and 1 more like this\n... and 6 more\n4 errors, 6 warnings"),
        "{out}"
    );
}

#[test]
fn text_output_escapes_control_characters() {
    let src = "diagram \"t\" size (100,100)\ntext t \"\u{1b}]52;c;aGk=\u{7}\u{9b}x\" at (90,50)\n";
    let out = kashika()
        .args(["check", "-", "--stdin-name", "a\u{1b}[2J.kash"])
        .write_stdin(src)
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        !text.chars().any(|c| c.is_control() && c != '\n'),
        "{text:?}"
    );
    assert!(text.contains("a\\u{1b}[2J.kash:2:1: warn["), "{text}");
    assert!(
        text.contains("text t \"\\u{1b}]52;c;aGk=\\u{7}\\u{9b}x\""),
        "{text}"
    );

    let out = kashika()
        .args(["check", "-", "--format", "json"])
        .write_stdin(src)
        .output()
        .unwrap();
    let json = String::from_utf8(out.stdout).unwrap();
    assert!(
        !json.chars().any(|c| c.is_control() && c != '\n'),
        "{json:?}"
    );
}

#[test]
fn tool_errors_escape_control_characters() {
    kashika()
        .args(["check", "missing\u{1b}]0;x\u{7}.kash"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("missing\\u{1b}]0;x\\u{7}.kash"));
}
