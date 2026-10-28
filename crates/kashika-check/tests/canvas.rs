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
