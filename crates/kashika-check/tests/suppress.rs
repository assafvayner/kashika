mod common;

use common::*;
use kashika_diag::Diagnostic;

const CROSSING: &str = "line a (0,50) -> h 100\nline b (50,0) -> v 100\nline c (0,70) -> h 100\n";

fn crosses(src: &str) -> Vec<Diagnostic> {
    kashika_check::check(&scene_src(src))
        .into_iter()
        .filter(|d| d.code == "cross")
        .collect()
}

#[test]
fn diagram_allow_silences_a_code_everywhere_without_a_reason() {
    let d = crosses(&format!(
        "diagram \"t\" size (200,200) allow=cross\n{CROSSING}"
    ));
    assert_eq!(d.len(), 2, "{d:#?}");
    assert!(d.iter().all(|d| d.suppressed && d.reason.is_none()));
    let near = kashika_check::check(&scene_src(&format!(
        "diagram \"t\" size (200,200) allow=near\n{CROSSING}"
    )));
    assert!(near.iter().any(|d| d.code == "cross" && !d.suppressed));
}

#[test]
fn diagram_allow_keeps_the_reason_an_element_gives() {
    let src = format!(
        "diagram \"t\" size (200,200) allow=cross\n{}",
        CROSSING.replace(
            "line a (0,50) -> h 100",
            "line a (0,50) -> h 100 allow=cross:b reason=\"bus\""
        )
    );
    let d = crosses(&src);
    assert!(d.iter().all(|d| d.suppressed), "{d:#?}");
    let reasons: Vec<Option<&str>> = d.iter().map(|d| d.reason.as_deref()).collect();
    assert!(reasons.contains(&Some("bus")), "{reasons:?}");
    assert!(reasons.contains(&None), "{reasons:?}");
}

#[test]
fn allow_code_other_silences_only_findings_naming_the_other() {
    let d = lint(
        "cross",
        &CROSSING.replace(
            "line b (50,0) -> v 100",
            "line b (50,0) -> v 100 allow=cross:a reason=\"tap\"",
        ),
    );
    assert_eq!(d.len(), 2, "{d:#?}");
    let with_a = d
        .iter()
        .find(|d| d.elements.iter().any(|e| e.id == "a"))
        .unwrap();
    assert!(with_a.suppressed);
    assert_eq!(with_a.reason.as_deref(), Some("tap"));
    let with_c = d
        .iter()
        .find(|d| d.elements.iter().any(|e| e.id == "c"))
        .unwrap();
    assert!(!with_c.suppressed);
    assert!(with_c.reason.is_none());
}

#[test]
fn allow_code_other_never_matches_the_owner_itself() {
    let d = lint(
        "cross",
        &CROSSING.replace(
            "line b (50,0) -> v 100",
            "line b (50,0) -> v 100 allow=cross:b",
        ),
    );
    assert!(d.iter().all(|d| !d.suppressed), "{d:#?}");
}

#[test]
fn allow_code_other_on_a_box() {
    let body = "box card at (0,0) size (100,100)\nbox badge at (80,80) size (40,40) allow=overlap:card reason=\"badge\"\nbox other at (110,110) size (40,40)\n";
    let d = lint("overlap", body);
    assert_eq!(d.len(), 2, "{d:#?}");
    let flags: Vec<(bool, Option<&str>)> = d
        .iter()
        .map(|d| (d.suppressed, d.reason.as_deref()))
        .collect();
    assert!(flags.contains(&(true, Some("badge"))), "{flags:?}");
    assert!(flags.contains(&(false, None)), "{flags:?}");
}
