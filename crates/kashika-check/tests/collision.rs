mod common;

use common::*;

#[test]
fn label_over_a_box_suggests_clear_positions() {
    let d = lint(
        "label-collision",
        "box b at (180,80) size (40,40)\nline l: (0,100) -> h 400 label=\"SQL\"\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "label \"SQL\" of l collides with box 'b'");
    assert_eq!(
        d[0].help,
        vec!["clear label positions on l: label@0.05..0.4, label@0.6..0.95".to_string()]
    );
}

#[test]
fn texts_lines_and_other_labels() {
    assert_eq!(
        lint(
            "label-collision",
            "line a: (0,100) -> h 400\ntext \"note\" at (200,100)\n"
        )
        .len(),
        1
    );
    assert_eq!(
        lint(
            "label-collision",
            "text \"one\" at (100,100)\ntext \"two\" at (105,102)\n"
        )
        .len(),
        1
    );
    assert!(lint(
        "label-collision",
        "line a: (0,100) -> h 400 label=\"x\"\ntext \"note\" at (200,50)\n"
    )
    .is_empty());
}

#[test]
fn group_titles_are_obstacles() {
    let d = lint(
        "label-collision",
        "group g \"VPC\" at (0,0) size (300,200) {\n}\ntext \"x\" at (20,17)\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "text 'text@L4' collides with the title of group 'g'"
    );
}
