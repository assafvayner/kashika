use std::io::Write;
use std::process::{Command, Stdio};

use kashika::api;

const CLEAN: &str = "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40)\n";
const WARN: &str = "diagram \"t\" size (300,200)\nbox a \"A\" at (20,20) size (80,40)\nbox b \"B\" at (200,20) size (80,40)\nbox c \"C\" at (110,20) size (80,40)\nline ab a.right -> b.left\n";
const BROKEN: &str = "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40)\nline x a.right -> nope.left\n";
const LIBRARY: &str = "style s { fill=\"#ffffff\" }\nbox a\n";

/// Runs the CLI on stdin with `args`, naming the input like the API does.
fn cli(src: &str, args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kashika"));
    cmd.env_remove("KASHIKA_ICON_PATH").args(args);
    if args.contains(&"-") {
        cmd.args(["--stdin-name", api::NAME]);
    }
    let mut child = cmd
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

#[test]
fn inspect_matches_the_cli() {
    let text = api::inspect(WARN, &[], false).unwrap().0;
    assert_eq!(text, cli(WARN, &["inspect", "-"]));
    let (text, _) = api::inspect(WARN, &["a".to_string()], true).unwrap();
    assert_eq!(text, cli(WARN, &["inspect", "-", "a", "--ports"]));
    let err = api::inspect(WARN, &["zz".to_string()], false)
        .err()
        .unwrap();
    assert!(err.starts_with("unknown id `zz`"), "{err}");
}

#[test]
fn route_matches_the_cli() {
    let (text, s) = api::route(
        WARN,
        Some("a.bottom"),
        Some("b.bottom"),
        "corner",
        None,
        8.0,
        3,
    )
    .unwrap();
    assert_eq!(text, cli(WARN, &["route", "-", "a.bottom", "b.bottom"]));
    assert!(!s.candidates.is_empty());
    assert!(matches!(
        api::route(
            WARN,
            Some("a.right"),
            Some("b.left"),
            "zigzag",
            None,
            8.0,
            3
        ),
        Err(api::RouteError::Usage(_))
    ));
    assert!(matches!(
        api::route(
            BROKEN,
            Some("a.right"),
            Some("a.left"),
            "corner",
            None,
            8.0,
            3
        ),
        Err(api::RouteError::Input(_))
    ));
    assert!(matches!(
        api::route(
            WARN,
            Some("a.right"),
            Some("zz.left"),
            "corner",
            None,
            8.0,
            3
        ),
        Err(api::RouteError::Endpoints(_))
    ));
}

#[test]
fn measure_matches_the_cli() {
    let m = api::measure("**Payments** API\\nline two", 14.0, false).unwrap();
    assert_eq!(m.lines, 2);
    assert_eq!(
        cli("", &["measure", "**Payments** API\\nline two"]).trim(),
        format!("width={} height={} lines={}", m.width, m.height, m.lines)
    );
    assert!(api::measure("**unclosed", 14.0, false).is_err());
}

#[test]
fn ref_and_icons() {
    assert!(api::ref_page(None).unwrap().contains("## Topics"));
    assert!(
        api::ref_page(Some("through"))
            .unwrap()
            .starts_with("# through")
    );
    assert!(
        api::ref_page(Some("nope"))
            .unwrap_err()
            .contains("unknown ref topic")
    );
    let all = api::icons(None);
    assert!(all.iter().any(|e| e.name == "cylinder"));
    let hits = api::icons(Some("database"));
    assert!(hits.iter().any(|e| e.name == "cylinder") && hits.len() < all.len());
}

#[test]
fn elements_for_hit_testing() {
    let src = "diagram \"t\" size (300,200)\ngroup g \"G\" at (10,10) size (200,150) {\n  box a \"A\" at (20,30) size (80,40)\n}\ntext t \"hi\" at (250,20)\n";
    let els = api::elements(src);
    let a = els.iter().find(|e| e.id == "a").unwrap();
    assert_eq!(
        (a.kind, a.parent.as_deref(), a.depth),
        ("box", Some("g"), 1)
    );
    assert_eq!(a.bbox, [[30.0, 40.0], [110.0, 80.0]]);
    assert!(els.iter().any(|e| e.id == "t" && e.kind == "text"));
    assert!(api::elements("box a\n").is_empty());
}

#[test]
fn inspect_escapes_control_characters_like_the_cli() {
    let src = "diagram \"t\" size (200,100)\nbox a \"A\u{1b}B\" at (20,20) size (80,40)\n";
    let text = api::inspect(src, &[], false).unwrap().0;
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert_eq!(text, cli(src, &["inspect", "-"]));
}
