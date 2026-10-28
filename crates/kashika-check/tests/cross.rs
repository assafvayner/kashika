mod common;

use common::*;

#[test]
fn crossing_lines_are_reported_on_the_later_line() {
    let d = lint(
        "cross",
        "line a (100,100) -> h 200\nline b (200,50) -> v 100\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "b crosses a at (200,100)");
    assert_eq!(d[0].span.line, 3);
    assert_eq!(d[0].elements[0].id, "b");
    assert_eq!(d[0].elements[0].segment, Some(1));
    assert_eq!(d[0].elements[1].id, "a");
    assert_eq!(d[0].facts["at"], serde_json::json!([200.0, 100.0]));
    assert_eq!(
        d[0].help,
        vec![
            "segment 1 runs at x=200; free x-intervals across y 50..150: 0..94, 306..800"
                .to_string(),
            "if intentional: allow=cross:a on b".to_string(),
        ]
    );
}

#[test]
fn shared_endpoints_and_junctions_are_not_crossings() {
    assert!(
        lint(
            "cross",
            "line a (0,0) -> (100,100)\nline b (100,100) -> (200,0)\n"
        )
        .is_empty()
    );
    assert!(lint(
        "cross",
        "junction j at (100,100)\nline a (0,100) -> h 200\nline b (100,0) -> j\nline c j -> v 100\n"
    )
    .is_empty());
}

#[test]
fn a_shared_segment_is_not_also_a_crossing() {
    assert!(
        lint(
            "cross",
            "line a (100,100) -> h 200\nline b (100,100) -> h 100 -> v 100\n"
        )
        .is_empty()
    );
}

#[test]
fn tangency_counts_once_and_curves_take_part() {
    let d = lint(
        "cross",
        "line a (0,100) -> h 200\nline b (50,50) -> curve right 50 (150,50)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    let d = lint(
        "cross",
        "line a (0,100) -> h 200\nline b (50,50) -> curve right 100 (150,50)\n",
    );
    assert_eq!(d.len(), 2, "{d:#?}");
}

#[test]
fn unnamed_lines_are_told_to_take_a_name() {
    let d = lint(
        "cross",
        "line (100,100) -> h 200\nline b (200,50) -> v 100\n",
    );
    assert_eq!(
        d[0].help.last().unwrap(),
        "if intentional: name line@L2 and add allow=cross:<its id> on b, or allow=cross on b"
    );
}
