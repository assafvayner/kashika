mod common;

use common::*;

#[test]
fn line_through_a_box_reports_entry_exit_and_free_lanes() {
    let d = lint(
        "through",
        "box a at (40,110) size (140,60)\nbox cache at (300,110) size (80,60)\nbox c at (520,110) size (100,60)\nline l a.right -> c.left\n",
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
        "box a at (0,0) size (100,100)\nbox b at (300,0) size (100,100)\nline l a.center -> b.center\n"
    )
    .is_empty());
    let d = lint(
        "through",
        "box a at (0,0) size (100,100)\nline l a.right -> h 50 -> v 150 -> x 50 -> y 50 -> h -100\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].elements[0].segment, Some(4));
}

#[test]
fn groups_are_never_obstacles() {
    assert!(
        lint(
            "through",
            "group g at (0,0) size (300,300) {\n}\nline l (10,150) -> h 200\n"
        )
        .is_empty()
    );
}

#[test]
fn reaching_an_edge_anchor_from_inside_its_own_box_is_flagged() {
    let d = lint(
        "through",
        "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100)\nline l a.right -> (240,45) -> vh b.left\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l runs through the inside of its own endpoint box 'b'"
    );
    assert_eq!(d[0].elements[1].id, "b");
    assert_eq!(d[0].facts["entry"], serde_json::json!([200.0, 45.0]));
    assert_eq!(d[0].facts["exit"], serde_json::json!([200.0, 70.0]));
    assert!(
        d[0].help[0].starts_with("arrive at 'b' from outside"),
        "{d:#?}"
    );

    let d = lint(
        "through",
        "box a at (20,20) size (100,100)\nbox b at (300,20) size (60,50)\nline l a.right -> h -40 -> v 150 -> hv b.bottom\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l runs through the inside of its own endpoint box 'a'"
    );
    assert_eq!(d[0].facts["entry"], serde_json::json!([120.0, 70.0]));
    assert_eq!(d[0].facts["exit"], serde_json::json!([80.0, 120.0]));
    assert!(d[0].help[0].starts_with("leave 'a' straight out"), "{d:#?}");
}

#[test]
fn normal_attachments_to_own_boxes_are_not_flagged() {
    let boxes = "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100)\n";
    for line in [
        "line l a.right -> (150,45) -> vh b.left end=arrow start=diamond",
        "line l a.top -> v -10 -> x 250 -> b.top",
        "line l a.bottom -> v 60 -> x 220 -> y 120 -> b.bottom",
        "line l a.center -> b.center",
        "line l a.center+(10,5) -> hv b.left",
    ] {
        let src = format!("{boxes}{line}\n");
        let d = lint("through", &src);
        assert!(d.is_empty(), "{src}: {d:#?}");
    }
    for src in [
        "box a at (20,20) size (60,50) icon=\"lock\"\nshape srv from \"server\" text-area=below\nbox b \"B\" at (200,20) size (100,60) shape=srv\nline l a.right -> b.left end=arrow\nline m a.bottom -> v 60 -> hv b.bottom\n",
        "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100) layer=under\nline l a.right -> (240,45) -> vh b.left\n",
    ] {
        let d = lint("through", src);
        assert!(d.is_empty(), "{src}: {d:#?}");
    }
}
