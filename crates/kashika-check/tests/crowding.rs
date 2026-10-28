mod common;

use common::*;

#[test]
fn endpoints_closer_than_8px_on_one_shape() {
    let d = lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50\nline l2 a.top@0.53 -> v -50\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l2 and l1 meet 'a' 3px apart (minimum 8px)");
    assert!(lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 a.top@0.3 -> v -50\nline l2 a.top@0.7 -> v -50\n"
    )
    .is_empty());
}

#[test]
fn identical_endpoints_crowd_only_when_both_draw_markers() {
    assert!(lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 (0,0) -> hv a.top\nline l2 (300,0) -> hv a.top\n"
    )
    .is_empty());
    let d = lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 (0,0) -> hv a.top end=arrow\nline l2 (300,0) -> hv a.top end=arrow\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l2 and l1 both draw a marker at (150,100) on 'a'"
    );
}

#[test]
fn junction_endpoints_are_exempt() {
    assert!(lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\njunction j at a.top\nline l1 (0,0) -> hv j end=arrow\nline l2 (300,0) -> hv j end=arrow\n"
    )
    .is_empty());
}
