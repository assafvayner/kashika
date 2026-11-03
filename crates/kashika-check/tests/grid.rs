mod common;

use common::*;
use kashika_diag::Span;

#[test]
fn off_grid_checks_authored_literals_only() {
    let s = scene_src(
        "diagram \"t\" size (800,600) grid=10\nbox a at (40,45) size (120,60)\nbox b at a.right+(33,0) size (100,60) pin=left\n",
    );
    let msgs: Vec<String> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "off-grid")
        .map(|d| d.message)
        .collect();
    assert_eq!(
        msgs,
        [
            "y=45 on 'a' is not a multiple of grid 10",
            "dx=33 on 'b' is not a multiple of grid 10",
        ]
    );
    let spans: Vec<Span> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "off-grid")
        .map(|d| d.span)
        .collect();
    assert_eq!(spans, [Span::new(2, 10, 2, 17), Span::new(3, 10, 3, 24)]);
    assert!(lint("off-grid", "box a at (40,45) size (120,60)\n").is_empty());
}
