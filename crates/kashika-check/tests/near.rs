mod common;

use common::*;

#[test]
fn line_passing_close_to_a_box() {
    let d = lint(
        "near",
        "box a at (100,100) size (100,60)\nline l (0,164) -> h 300\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l runs 4px from box 'a' (clearance 6)");
    assert_eq!(d[0].facts["distance"], serde_json::json!(4.0));
}

#[test]
fn running_along_its_own_box_edge() {
    let d = lint(
        "near",
        "box a at (100,100) size (100,60)\nline l a.tl+(10,0) -> h 80 -> v -50\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l runs along the edge of its own box 'a', 0px away (clearance 6)"
    );
    assert!(lint(
        "near",
        "box a at (100,100) size (100,60)\nbox b at (400,100) size (100,60)\nline l a.right -> b.left\n"
    )
    .is_empty());
}

#[test]
fn a_line_through_a_box_is_not_also_near_it() {
    assert!(
        lint(
            "near",
            "box a at (100,100) size (100,60)\nline l (0,130) -> h 300\n"
        )
        .is_empty()
    );
}

#[test]
fn parallel_lines_closer_than_clearance_for_over_20px() {
    let d = lint("near", "line a (0,100) -> h 300\nline b (0,104) -> h 300\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "b runs 4px from a for 300px (clearance 6)");
    assert!(lint("near", "line a (0,100) -> h 300\nline b (0,104) -> h 15\n").is_empty());
    assert!(lint("near", "line a (0,100) -> h 300\nline b (0,110) -> h 300\n").is_empty());
}

#[test]
fn clearance_comes_from_the_diagram() {
    let s = scene_src(
        "diagram \"t\" size (800,600) clearance=12\nline a (0,100) -> h 300\nline b (0,110) -> h 300\n",
    );
    assert_eq!(
        kashika_check::check(&s)
            .iter()
            .filter(|d| d.code == "near")
            .count(),
        1
    );
}

#[test]
fn running_along_a_group_border() {
    let g = "group vpc \"VPC\" at (200,20) size (300,300) {\n}\n";
    let d = lint("near", &format!("{g}line l (100,100) -> x 200 -> v 100\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l runs along the border of group 'vpc' (clearance 6)"
    );
    assert_eq!(d[0].elements[0].segment, Some(2));
    assert_eq!(
        d[0].help,
        vec!["segment 2 runs at x=200; free x-intervals across y 100..200: 0..194, 206..494, 506..800".to_string()]
    );
    let d = lint("near", &format!("{g}line l (100,316) -> h 300\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "l runs 4px from the border of group 'vpc' (clearance 6)"
    );
}

#[test]
fn crossing_a_group_border_is_not_near_it() {
    let g = "group vpc \"VPC\" at (200,20) size (300,300) {\n}\n";
    assert!(lint("near", &format!("{g}line l (100,200) -> h 200 -> v 50\n")).is_empty());
    assert!(lint("near", &format!("{g}line l (180,100) -> v 100\n")).is_empty());
}
