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
