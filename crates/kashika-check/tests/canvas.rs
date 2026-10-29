mod common;

use common::*;

#[test]
fn shapes_and_lines_beyond_the_canvas() {
    let d = lint(
        "out-of-canvas",
        "box a at (750,10) size (100,60)\nline l (10,10) -> v 700\n",
    );
    assert_eq!(d.len(), 2, "{d:#?}");
    assert_eq!(
        d[0].message,
        "box 'a' extends beyond the 800x600 canvas (right by 50px)"
    );
    assert_eq!(
        d[1].message,
        "line l extends beyond the 800x600 canvas (bottom by 110px)"
    );
    assert!(lint("out-of-canvas", "box a at (700,540) size (100,60)\n").is_empty());
}

#[test]
fn content_above_or_left_of_the_origin_must_move() {
    let d = lint("out-of-canvas", "box a at (750,10) size (100,60)\n");
    assert_eq!(
        d[0].help,
        vec!["move it inside (0,0)-(800,600) or enlarge `size` on the `diagram` line"]
    );
    let s = scene_src("diagram \"t\"\nbox a at (-30,20) size (100,40)\n");
    let d: Vec<_> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "out-of-canvas")
        .collect();
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].help.len(), 1, "{d:#?}");
    assert!(d[0].help[0].starts_with("move it right by 30px"), "{d:#?}");
    assert!(
        d[0].help[0].ends_with("a fitted canvas grows only right and down"),
        "{d:#?}"
    );
}

#[test]
fn children_of_an_escaping_group_are_not_reported_again() {
    let d = lint(
        "out-of-canvas",
        "group g \"G\" at (700,10) size (200,100) {\n  box a at (10,30) size (180,40)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.starts_with("group 'g'"), "{d:#?}");
    let d = lint(
        "out-of-canvas",
        "group g \"G\" at (100,10) size (200,100) {\n  box a at (750,30) size (80,40)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.starts_with("box 'a'"), "{d:#?}");
}
