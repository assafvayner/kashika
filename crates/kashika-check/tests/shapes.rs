mod common;

use common::*;

const DIAMOND: &str = "box d at (100,100) size (100,100) shape=diamond\n";
const CYLINDER: &str = "box db at (100,100) size (100,100) shape=cylinder\n";

#[test]
fn lines_through_a_diamond_follow_its_outline() {
    let d = lint(
        "through",
        &format!("{DIAMOND}line l (0,150) -> (300,150)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l passes through box 'd'");
    assert_eq!(d[0].facts["entry"], serde_json::json!([100.0, 150.0]));
    assert!(lint(
        "through",
        &format!("{DIAMOND}line l (90,110) -> (110,90)\n")
    )
    .is_empty());
}

#[test]
fn near_measures_to_the_diamond_edge() {
    let d = lint("near", &format!("{DIAMOND}line l (100,145) -> (145,100)\n"));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["distance"], serde_json::json!(3.54));
    assert!(lint("near", &format!("{DIAMOND}line l (90,140) -> (140,90)\n")).is_empty());
}

#[test]
fn diamonds_overlap_only_when_their_outlines_do() {
    let a = "box a at (0,0) size (100,100) shape=diamond\n";
    assert!(lint(
        "overlap",
        &format!("{a}box b at (80,80) size (100,100) shape=diamond\n")
    )
    .is_empty());
    let d = lint(
        "overlap",
        &format!("{a}box b at (40,40) size (100,100) shape=diamond\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
}

#[test]
fn endpoints_snap_to_the_cylinder_cap() {
    let d = lint(
        "endpoint-off-edge",
        &format!("{CYLINDER}line l (150,40) -> (150,97)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["nearest"], serde_json::json!([150.0, 100.0]));
    assert_eq!(
        d[0].help,
        vec!["snap it to the outline with an anchor: `db.top`"]
    );
    assert!(lint(
        "endpoint-off-edge",
        &format!("{CYLINDER}line l (110,40) -> (110,108)\n")
    )
    .is_empty());
}

#[test]
fn anchored_lines_on_built_in_shapes_check_clean() {
    let s = scene(&format!(
        "{CYLINDER}box d at (300,100) size (100,100) shape=diamond\n\
         line a d.left -> db.right end=arrow\n\
         line b db.top@0.2 -> v -60 end=arrow\n\
         line c d.top@0.3 -> v -60 end=arrow\n"
    ));
    assert!(kashika_check::check(&s).is_empty());
    let ctx = kashika_check::ctx::Ctx::new(&s);
    let ports: Vec<_> = kashika_check::ports::ports(&ctx)
        .into_iter()
        .map(|p| format!("{}.{}@{}", p.shape, p.side, (p.t * 100.0).round() / 100.0))
        .collect();
    assert_eq!(
        ports,
        ["db.right@0.5", "db.top@0.2", "d.left@0.5", "d.top@0.3"]
    );
}

const LOGO: &str = "image logo \"logo.png\" at (100,100) size (100,50)\n";

#[test]
fn images_are_opaque_obstacles() {
    let dir = tempfile::tempdir().unwrap();
    let d = lint_in(
        dir.path(),
        "through",
        &format!("{LOGO}line l (0,125) -> (300,125)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l passes through image 'logo'");
    assert_eq!(d[0].elements[1].kind, "image");
    assert!(d[0].help[0].contains("0..94, 156..600"), "{:?}", d[0].help);
    let d = lint_in(
        dir.path(),
        "near",
        &format!("{LOGO}line l (0,97) -> (300,97)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l runs 3px from image 'logo' (clearance 6)");
    let d = lint_in(
        dir.path(),
        "overlap",
        &format!("{LOGO}box b at (180,120) size (60,60)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "box 'b' overlaps image 'logo'");
    let d = lint_in(
        dir.path(),
        "label-collision",
        &format!("{LOGO}line l (0,125) -> (300,125) label=\"to logo\"\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message.ends_with("collides with image 'logo'"),
        "{}",
        d[0].message
    );
    let d = lint_in(
        dir.path(),
        "out-of-canvas",
        "image logo \"logo.png\" at (760,0) size (80,40)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.starts_with("image 'logo' extends beyond"));
}

#[test]
fn lines_may_attach_to_images() {
    let dir = tempfile::tempdir().unwrap();
    for code in ["through", "near", "endpoint-off-edge"] {
        assert!(lint_in(
            dir.path(),
            code,
            &format!("{LOGO}box b at (400,100) size (100,50)\nline l logo.right -> b.left\n"),
        )
        .is_empty());
    }
}

const BELOW: &str = "shape srv from \"server\" text-area=below\n";

#[test]
fn labels_below_a_shape_are_checked_where_they_sit() {
    let d = lint(
        "out-of-canvas",
        &format!("{BELOW}box s \"Uploads\" at (100,530) size (60,60) shape=srv\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message
            .starts_with("the label under box 's' extends beyond the 800x600 canvas"),
        "{}",
        d[0].message
    );
    let body = format!("{BELOW}box s \"Uploads\" at (100,100) size (60,60) shape=srv\n");
    let d = lint(
        "label-collision",
        &format!("{body}line l (0,177) -> (300,177)\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "the label under box 's' collides with line l");
    assert!(lint("label-collision", &body).is_empty());
    let d = lint(
        "spill",
        &format!("{BELOW}group g at (80,80) size (120,100) {{\n  box s \"Uploads\" at (20,20) size (60,60) shape=srv\n}}\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0]
        .message
        .starts_with("the label under box 's' extends outside group 'g'"));
}

#[test]
fn labels_below_a_shape_sit_on_the_canvas_not_the_fill() {
    let d = lint(
        "low-contrast",
        &format!("{BELOW}box s \"Uploads\" at (100,100) size (60,60) shape=srv fill=#000000 color=#ffffff\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], serde_json::json!("#ffffff"));
    assert!(lint(
        "low-contrast",
        "box s \"Uploads\" at (100,100) size (120,60) fill=#000000 color=#ffffff\n"
    )
    .is_empty());
}

#[test]
fn icons_shrink_the_text_area_without_warnings() {
    let s = scene(
        "box q \"Jobs\" at (100,100) size (160,60) icon=\"queue\" style=queue\n\
         box w \"Web\" at (100,200) size (160,80) icon=\"browser\" icon-pos=top\n\
         line l q.bottom -> w.top end=arrow\n",
    );
    assert!(kashika_check::check(&s).is_empty());
    assert!(s.shapes[0].text_area.x >= 100.0 + 24.0);
}
