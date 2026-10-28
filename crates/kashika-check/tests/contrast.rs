mod common;

use common::*;

#[test]
fn light_text_on_a_light_fill() {
    let d = lint(
        "low-contrast",
        "box a \"API\" at (0,0) size (100,60) fill=#ffffff color=#dddddd\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "label \"API\" of 'a' has contrast 1.4:1 against its background (minimum 3:1)"
    );
    assert_eq!(d[0].facts["background"], "#ffffff");
    assert!(lint(
        "low-contrast",
        "box a \"API\" at (0,0) size (100,60) style=service\n"
    )
    .is_empty());
    assert!(lint(
        "low-contrast",
        "box a at (0,0) size (100,60) fill=#ffffff color=#ffffff\n"
    )
    .is_empty());
}

#[test]
fn text_takes_the_fill_of_the_shapes_under_it() {
    assert_eq!(
        lint(
            "low-contrast",
            "box a at (0,0) size (200,100) fill=#000000\ntext \"note\" at (100,50) color=#111111\n"
        )
        .len(),
        1
    );
    assert!(lint(
        "low-contrast",
        "box a at (0,0) size (200,100) fill=#000000\ntext \"note\" at (100,50) color=#ffffff\n"
    )
    .is_empty());
}

#[test]
fn line_labels_sit_on_a_white_halo() {
    assert_eq!(
        lint(
            "low-contrast",
            "style faint { color=#eeeeee }\nline l: (0,0) -> h 100 label=\"x\" style=faint\n"
        )
        .len(),
        1
    );
}

#[test]
fn sublabels_use_the_muted_color() {
    let d = lint(
        "low-contrast",
        "box a \"API\" at (0,0) size (100,60) sublabel=\"v2\" fill=#64748b\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(
        d[0].message.starts_with("sublabel \"v2\" of 'a'"),
        "{}",
        d[0].message
    );
}
