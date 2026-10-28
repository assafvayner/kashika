mod common;

use common::*;

#[test]
fn line_through_a_box_reports_entry_exit_and_free_lanes() {
    let d = lint(
        "through",
        "box a at (40,110) size (140,60)\nbox cache at (300,110) size (80,60)\nbox c at (520,110) size (100,60)\nline l: a.right -> c.left\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l passes through box 'cache'");
    assert_eq!(d[0].facts["entry"], serde_json::json!([300.0, 140.0]));
    assert_eq!(d[0].facts["exit"], serde_json::json!([380.0, 140.0]));
    assert_eq!(
        d[0].elements[1].bbox,
        Some([[300.0, 110.0], [380.0, 170.0]])
    );
    assert_eq!(
        d[0].help,
        vec![
            "segment 1 runs at y=140; free y-intervals across x 180..520: 0..104, 176..600"
                .to_string()
        ]
    );
}

#[test]
fn own_boxes_are_exempt_only_on_the_first_and_last_segments() {
    assert!(lint(
        "through",
        "box a at (0,0) size (100,100)\nbox b at (300,0) size (100,100)\nline l: a.center -> b.center\n"
    )
    .is_empty());
    let d = lint(
        "through",
        "box a at (0,0) size (100,100)\nline l: a.right -> h 50 -> v 150 -> x 50 -> y 50 -> h -100\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].elements[0].segment, Some(4));
}

#[test]
fn groups_are_never_obstacles() {
    assert!(lint(
        "through",
        "group g at (0,0) size (300,300) {\n}\nline l: (10,150) -> h 200\n"
    )
    .is_empty());
}
