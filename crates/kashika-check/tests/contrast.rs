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
    assert!(
        lint(
            "low-contrast",
            "box a \"API\" at (0,0) size (100,60) style=service\n"
        )
        .is_empty()
    );
    assert!(
        lint(
            "low-contrast",
            "box a at (0,0) size (100,60) fill=#ffffff color=#ffffff\n"
        )
        .is_empty()
    );
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
    assert!(
        lint(
            "low-contrast",
            "box a at (0,0) size (200,100) fill=#000000\ntext \"note\" at (100,50) color=#ffffff\n"
        )
        .is_empty()
    );
}

#[test]
fn aligned_text_takes_the_fill_under_its_center_not_its_anchor() {
    let dark = "box a at (104,0) size (200,100) fill=#000000\n";
    assert!(
        lint(
            "low-contrast",
            &format!("{dark}text \"a long note\" at (100,50) align=left color=#ffffff\n")
        )
        .is_empty()
    );
    let d = lint(
        "low-contrast",
        &format!("{dark}text \"a long note\" at (100,50) align=left color=#111111\n"),
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], "#000000");
    assert!(
        lint(
            "low-contrast",
            &format!("{dark}text \"a long note\" at (308,50) align=right color=#ffffff\n")
        )
        .is_empty()
    );
}

#[test]
fn line_labels_on_a_bare_canvas_count_as_white() {
    assert_eq!(
        lint(
            "low-contrast",
            "style faint { color=#eeeeee }\nline l (0,0) -> h 100 label=\"x\" style=faint\n"
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

#[test]
fn canvas_text_is_checked_against_the_background() {
    let on = |bg: &str, color: &str| {
        let src =
            format!("diagram \"t\" size (400,200){bg}\ntext \"note\" at (100,50) color={color}\n");
        kashika_check::check(&scene_src(&src))
            .into_iter()
            .filter(|d| d.code == "low-contrast")
            .collect::<Vec<_>>()
    };
    assert_eq!(on("", "#eeeeee").len(), 1);
    let d = on(" background=#ffffff", "#eeeeee");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], "#ffffff");
    assert!(on(" background=#0f172a", "#eeeeee").is_empty());
    let d = on(" background=#0f172a", "#111111");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], "#0f172a");
    assert!(on(" background=none", "#111111").is_empty());
}

#[test]
fn line_labels_take_the_fill_beneath_them() {
    let src = |fill: &str| {
        format!(
            "style pale {{ color=#f8fafc }}\ngroup g at (0,0) size (300,100) fill={fill} {{\n  line l (20,50) -> (280,50) label=\"x\" style=pale\n}}\n"
        )
    };
    assert!(lint("low-contrast", &src("#0f172a")).is_empty());
    assert_eq!(lint("low-contrast", &src("#ffffff")).len(), 1);
    let src = "diagram \"t\" size (400,200) background=#0f172a\nstyle pale { color=#f8fafc }\nline l (0,50) -> h 100 label=\"x\" style=pale\n";
    assert!(
        kashika_check::check(&scene_src(src))
            .iter()
            .all(|d| d.code != "low-contrast")
    );
}

#[test]
fn named_colors_are_checked_like_hex() {
    let d = lint(
        "low-contrast",
        "text \"note\" at (100,50) color=lightgray\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    let src = "diagram \"t\" size (400,200) background=white\ntext \"note\" at (100,50) color=lightgray\n";
    let d = kashika_check::check(&scene_src(src));
    let d: Vec<_> = d.iter().filter(|d| d.code == "low-contrast").collect();
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].facts["background"], "#ffffff");
}
