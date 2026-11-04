use std::io::Write;
use std::process::{Command, Stdio};

use kashika::api;

const CLEAN: &str = "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40)\n";
const WARN: &str = "diagram \"t\" size (300,200)\nbox a \"A\" at (20,20) size (80,40)\nbox b \"B\" at (200,20) size (80,40)\nbox c \"C\" at (110,20) size (80,40)\nline ab a.right -> b.left\n";
const BROKEN: &str = "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40)\nline x a.right -> nope.left\n";
const LIBRARY: &str = "style s { fill=\"#ffffff\" }\nbox a\n";

/// Runs the CLI on stdin with `args`, naming the input like the API does.
fn cli(src: &str, args: &[&str]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kashika"))
        .env_remove("KASHIKA_ICON_PATH")
        .args(args)
        .args(["--stdin-name", api::NAME])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(src.as_bytes())
        .unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
}

#[test]
fn check_matches_the_cli_text_report() {
    for src in [CLEAN, WARN, BROKEN, LIBRARY] {
        assert_eq!(api::check(src).text, cli(src, &["check", "-"]), "{src}");
    }
}

#[test]
fn check_counts() {
    let c = api::check(WARN);
    assert_eq!(c.errors, 0);
    assert!(c.warnings >= 1, "{}", c.text);
    assert_eq!(c.diagnostics.len(), c.warnings);
    assert_eq!(api::check(BROKEN).errors, 1);
}

#[test]
fn format_reformats_or_reports() {
    assert_eq!(
        api::format("diagram \"t\"  size (200,100)\n").unwrap(),
        "diagram \"t\" size (200, 100)\n"
    );
    let err = api::format("box a \"A\" at (\n").unwrap_err();
    assert!(err.contains("error[parse]"), "{err}");
}

#[test]
fn format_error_matches_the_cli_report() {
    let src = "box a \"A\" at (\n";
    assert_eq!(api::format(src).unwrap_err(), cli(src, &["fmt", "-"]));
}
