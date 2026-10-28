mod common;

use common::*;

#[test]
fn label_over_a_box_suggests_clear_positions() {
    let d = lint(
        "label-collision",
        "box b at (180,80) size (40,40)\nline l (0,100) -> h 400 label=\"SQL\"\n",
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
            "line a (0,100) -> h 400\ntext \"note\" at (200,100)\n"
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
    assert!(
        lint(
            "label-collision",
            "line a (0,100) -> h 400 label=\"x\"\ntext \"note\" at (200,50)\n"
        )
        .is_empty()
    );
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

#[test]
fn label_over_its_own_arrowhead() {
    let d = lint(
        "label-collision",
        "line l (20,100) -> h 40 end=arrow label=\"hello\"\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "label \"hello\" of l collides with the end arrowhead of l"
    );
    assert_eq!(d[0].elements[1].id, "l");
    assert_eq!(
        d[0].help,
        vec!["clear label positions on l: label@0.05..0.4".to_string()]
    );
    let d = lint(
        "label-collision",
        "line l (20,100) -> h 40 start=diamond label@0.2=\"hello\"\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "label \"hello\" of l collides with the start diamond of l"
    );
}

#[test]
fn label_on_its_own_stroke_away_from_markers() {
    assert!(lint(
        "label-collision",
        "line l (20,100) -> h 300 -> v 100 start=dot end=arrow label=\"hello\" label@0.9=\"x\"\n"
    )
    .is_empty());
}

#[test]
fn label_over_the_boxes_its_line_connects() {
    let d = lint(
        "label-collision",
        "box a at (20,80) size (80,40)\nbox b at (140,80) size (80,40)\nline l a.right -> b.left label=\"calls the api\"\n",
    );
    let hits: Vec<_> = d.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        hits,
        [
            "label \"calls the api\" of l collides with box 'a'",
            "label \"calls the api\" of l collides with box 'b'"
        ]
    );
}
