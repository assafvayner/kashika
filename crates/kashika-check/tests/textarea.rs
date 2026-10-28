mod common;

use common::*;

#[test]
fn item_past_the_right_edge() {
    let d = lint(
        "text-outside-shape",
        "box e at (220,120) size (160,60) {\n  text \"in\" at (20,20)\n  text \"out\" at (150,30) align=left\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].span.line, 4);
    assert!(
        d[0].message
            .starts_with("block item \"out\" lies outside the text area of box 'e' (right by "),
        "{}",
        d[0].message
    );
    assert_eq!(d[0].elements[0].id, "e");
    assert_eq!(
        d[0].help,
        vec!["keep the item within the text area, local (8,8)..(152,52)".to_string()]
    );
}

#[test]
fn items_inside_the_text_area() {
    assert!(lint(
        "text-outside-shape",
        "box e at (220,120) size (160,60) {\n  text \"in\" at (80,30)\n  text \"left\" at (8,20) align=left\n}\n",
    )
    .is_empty());
}

#[test]
fn a_diamond_text_area_is_its_center() {
    let d = lint(
        "text-outside-shape",
        "box d at (20,20) size (200,100) shape=diamond {\n  text \"x\" at (20,50)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.contains("(left by "), "{}", d[0].message);
}

#[test]
fn allow_on_the_box_suppresses_it() {
    let s = scene(
        "box e at (20,20) size (160,60) allow=text-outside-shape {\n  text \"out\" at (150,30)\n}\n",
    );
    let d: Vec<_> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "text-outside-shape")
        .collect();
    assert_eq!(d.len(), 1);
    assert!(d[0].suppressed);
}
