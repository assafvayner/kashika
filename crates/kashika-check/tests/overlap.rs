mod common;

use common::*;

#[test]
fn overlapping_boxes() {
    let d = lint(
        "overlap",
        "box a at (0,0) size (100,100)\nbox b at (50,50) size (100,100)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "box 'b' overlaps box 'a'");
    assert_eq!(
        d[0].facts["intersection"],
        serde_json::json!([[50.0, 50.0], [100.0, 100.0]])
    );
    assert!(
        lint(
            "overlap",
            "box a at (0,0) size (100,100)\nbox b at (100,0) size (100,100)\n"
        )
        .is_empty()
    );
}

#[test]
fn containment_is_overlap_unless_the_container_is_a_group() {
    let d = lint(
        "overlap",
        "box a at (0,0) size (100,100)\nbox b at (10,10) size (20,20)\n",
    );
    assert_eq!(d.len(), 1);
    assert!(
        d[0].help[0].starts_with("only a group may contain"),
        "{d:#?}"
    );
    assert!(lint(
        "overlap",
        "group g at (0,0) size (100,100) {\n  box b at (10,30) size (20,20)\n}\nbox c at (50,50) size (20,20)\n"
    )
    .is_empty());
    assert_eq!(
        lint(
            "overlap",
            "group g at (0,0) size (100,100) {\n}\nbox c at (90,50) size (20,20)\n"
        )
        .len(),
        1
    );
}

#[test]
fn shape_allow_suppresses_overlap() {
    let s = scene(
        "box a at (0,0) size (100,100)\nbox b at (50,50) size (100,100) allow=overlap:a reason=\"badge\"\n",
    );
    let d: Vec<_> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "overlap")
        .collect();
    assert!(d[0].suppressed);
    assert_eq!(d[0].reason.as_deref(), Some("badge"));
}
