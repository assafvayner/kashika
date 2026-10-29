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
fn label_beside_its_line_clears_the_arrowhead() {
    assert!(
        lint(
            "label-collision",
            "line l (20,100) -> h 40 end=arrow label=\"hello\" label-side=above\n",
        )
        .is_empty()
    );
}

#[test]
fn label_beside_its_line_collides_with_its_other_segments() {
    let d = lint(
        "label-collision",
        "line l (20,100) -> h 100 -> v -100 label@0.45=\"hello\" label-side=above\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "label \"hello\" of l collides with line l");
    assert_eq!(
        d[0].help,
        vec!["clear label positions on l: label@0.05..0.4".to_string()]
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
fn markers_on_thick_lines_reach_further() {
    let line = |w: &str, end: &str| {
        format!("line l (20,100) -> h 300 end={end} stroke-width={w} label@0.9=\"x\"\n")
    };
    assert!(lint("label-collision", &line("1.5", "arrow")).is_empty());
    for (end, name) in [
        ("arrow", "arrowhead"),
        ("triangle", "triangle"),
        ("circle", "circle"),
    ] {
        let d = lint("label-collision", &line("6", end));
        assert_eq!(d.len(), 1, "{d:#?}");
        assert_eq!(
            d[0].message,
            format!("label \"x\" of l collides with the end {name} of l")
        );
    }
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

#[test]
fn labels_on_group_borders() {
    let groups = "group a \"A\" at (0,0) size (200,200) {\n}\ngroup b \"B\" at (230,0) size (200,200) {\n}\n";
    let d = lint(
        "label-collision",
        &format!("{groups}line l (100,100) -> h 230 label=\"HTTPS\"\n"),
    );
    assert_eq!(d.len(), 2, "{d:#?}");
    assert_eq!(
        d[0].message,
        "label \"HTTPS\" of l collides with the border of group 'a'"
    );
    assert_eq!(
        d[1].message,
        "label \"HTTPS\" of l collides with the border of group 'b'"
    );
    assert_eq!(
        d[0].help,
        vec!["clear label positions on l: label@0.05..0.35, label@0.65..0.95".to_string()]
    );
    assert!(
        lint(
            "label-collision",
            &format!("{groups}line l (100,100) -> h 230 label@0.2=\"HTTPS\"\n"),
        )
        .is_empty()
    );
    assert!(
        lint(
            "label-collision",
            &format!("{groups}text \"inside\" at (100,100)\ntext \"x\" at (215,150)\n"),
        )
        .is_empty()
    );
}

#[test]
fn lines_crossing_group_titles() {
    let group = "group pub \"public subnet\" at (0,0) size (300,200) {\n  box nat \"NAT\" at (20,60) size (80,40)\n}\n";
    let d = lint(
        "label-collision",
        &format!("{group}line l nat.top -> y -40\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].message, "l crosses the title of group 'pub'");
    assert_eq!(d[0].elements[1].id, "pub");
    assert!(
        d[0].help[0].starts_with("the title of 'pub' covers x 8.."),
        "{:?}",
        d[0].help
    );
    assert_eq!(
        d[0].help[1],
        "or move the title away from the line: `text-align=right` on 'pub'"
    );
    assert!(
        lint(
            "label-collision",
            &format!(
                "{}line l nat.top -> y -40\n",
                group.replacen("size (300,200)", "size (300,200) text-align=right", 1)
            ),
        )
        .is_empty()
    );
    assert!(
        lint(
            "label-collision",
            &format!("{group}line l nat.right -> h 100 -> v -100\n"),
        )
        .is_empty()
    );
}

#[test]
fn label_over_its_own_crows_foot() {
    let d = lint(
        "label-collision",
        "line l (20,100) -> h 60 start=zero-many label@0.15=\"x\"\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "label \"x\" of l collides with the start crow's foot of l"
    );
    let d = lint(
        "label-collision",
        "line l (20,100) -> h 60 end=one label@0.85=\"x\"\n",
    );
    assert_eq!(
        d.iter().map(|d| d.message.as_str()).collect::<Vec<_>>(),
        ["label \"x\" of l collides with the end bar of l"]
    );
    assert!(
        lint(
            "label-collision",
            "line l (20,100) -> h 200 start=zero-many end=one-many label=\"x\"\n"
        )
        .is_empty()
    );
}
