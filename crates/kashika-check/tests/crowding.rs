mod common;

use common::*;
use kashika_diag::Span;
use kashika_model::Marker;

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

#[test]
fn exactly_8px_apart_is_not_crowded() {
    let d = lint(
        "port-crowding",
        "box a at (100,200) size (100,60)\nbox b at (500,200) size (100,60)\nline a1 a.top@0.5 -> v -100\nline a2 a.top@0.58 -> v -100\nline b1 b.top@0.5 -> v -100\nline b2 b.top@0.579 -> v -100\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "b2 and b1 meet 'b' 7.9px apart (minimum 8px)");
}

#[test]
fn minimum_gap_per_marker_follows_its_half_width() {
    for (end, min) in [
        ("none", 8),
        ("arrow", 16),
        ("open", 16),
        ("triangle", 24),
        ("diamond", 16),
        ("hollow-diamond", 16),
        ("dot", 12),
        ("circle", 16),
        ("one", 24),
        ("many", 24),
        ("one-many", 24),
        ("zero-one", 24),
        ("zero-many", 24),
    ] {
        let m = Marker::from_name(end).unwrap();
        assert_eq!(f64::from(min), 8f64.max(2.0 * m.half_width(3.0)), "{end}");
        let d = lint(
            "port-crowding",
            &format!(
                "box a at (100,100) size (400,60)\nline l1 a.top@0.5 -> v -50 start={end} stroke-width=3\nline l2 a.top@0.5125 -> v -50 start={end} stroke-width=3\n"
            ),
        );
        assert_eq!(d.len(), 1, "{end}: {d:#?}");
        assert_eq!(
            d[0].message,
            format!("l2 and l1 meet 'a' 5px apart (minimum {min}px)"),
            "{end}"
        );
        let end_col = 52 + u32::try_from(end.len()).unwrap();
        assert_eq!(d[0].span, Span::new(4, 1, 4, end_col), "{end}");
    }
}
