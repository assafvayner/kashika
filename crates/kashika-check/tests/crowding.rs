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

#[test]
fn crows_feet_need_room_for_both_spreads() {
    let d = lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=many\nline l2 a.top@0.6 -> v -50 start=one-many\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l2 and l1 meet 'a' 10px apart (minimum 12px)");
    assert!(d[0].help[0].starts_with("spread the endpoints at least 12px apart"));
    assert!(lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=many\nline l2 a.top@0.62 -> v -50 start=zero-many\n"
    )
    .is_empty());
    assert!(lint(
        "port-crowding",
        "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=arrow\nline l2 a.top@0.6 -> v -50 start=arrow\n"
    )
    .is_empty());
}

#[test]
fn markers_on_thick_lines_need_more_room() {
    let body = |w: &str| {
        format!(
            "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=arrow stroke-width={w}\nline l2 a.top@0.6 -> v -50 start=arrow stroke-width={w}\n"
        )
    };
    assert!(lint("port-crowding", &body("1.5")).is_empty());
    let d = lint("port-crowding", &body("3"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l2 and l1 meet 'a' 10px apart (minimum 16px)");
}
