mod common;

use common::*;

#[test]
fn an_absolute_item_over_the_label() {
    let d = lint(
        "text-overlap",
        "box p \"Cache pod\" at (20,20) size (170,86) {\n  text \"RAM size cache\" at (85,46)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 3);
    assert!(
        d[0].message
            .starts_with("block item \"RAM size cache\" overlaps the label of box 'p' by "),
        "{}",
        d[0].message
    );
    assert_eq!(d[0].elements[0].id, "p");
    assert_eq!(
        d[0].help,
        vec!["move it with `at`, or drop the label and put it in the block".to_string()]
    );
}

#[test]
fn two_absolute_items_over_each_other_once() {
    let d = lint(
        "text-overlap",
        "box p at (20,20) size (170,86) {\n  text \"one\" at (85,40)\n  text \"two\" at (90,44)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 4);
    assert!(
        d[0].message
            .starts_with("block item \"two\" overlaps block item \"one\" of box 'p'"),
        "{}",
        d[0].message
    );
    assert_eq!(d[0].help, vec!["move one of them with `at`".to_string()]);
}

#[test]
fn items_apart_from_the_label_and_each_other() {
    assert!(lint(
        "text-overlap",
        "box p \"Cache pod\" at (20,20) size (170,86) {\n  text \"RAM · L1\" at (85,70) font-size=11\n}\nbox q at (220,20) size (170,86) {\n  text \"one\" at (85,30)\n  text \"two\" at (85,50)\n}\n",
    )
    .is_empty());
}

#[test]
fn allow_on_the_box_suppresses_it() {
    let s = scene(
        "box p \"Cache pod\" at (20,20) size (170,86) allow=text-overlap {\n  text \"RAM\" at (85,46)\n}\n",
    );
    let d: Vec<_> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "text-overlap")
        .collect();
    assert_eq!(d.len(), 1);
    assert!(d[0].suppressed);
}
