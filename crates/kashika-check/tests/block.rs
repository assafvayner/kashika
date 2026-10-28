mod common;

use common::*;

#[test]
fn label_and_muted_sublabel() {
    let d = lint(
        "block-reducible",
        "box api at (20,20) size (160,60) {\n  text \"Payments \\\"API\\\"\"\n  text \"v2.4.1\" size=11.2 muted\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "the text block of box 'api' can be written as label/sublabel"
    );
    assert_eq!(d[0].span.line, 3);
    assert_eq!(
        d[0].help,
        vec![
            r#"drop the block and write `box api "Payments \"API\"" sublabel="v2.4.1"`"#
                .to_string()
        ]
    );
}

#[test]
fn one_item_or_two_lines_keep_their_alignment() {
    let d = lint(
        "block-reducible",
        "box a at (20,20) size (160,60) text-align=left {\n  text \"one\"\n}\n",
    );
    assert_eq!(
        d[0].help,
        vec!["drop the block and write `box a \"one\" text-align=left`".to_string()]
    );
    let d = lint(
        "block-reducible",
        "box a at (20,20) size (160,60) {\n  text \"one\"\n  text \"two\"\n}\n",
    );
    assert_eq!(
        d[0].help,
        vec!["drop the block and write `box a \"one\\ntwo\"`".to_string()]
    );
}

#[test]
fn blocks_a_label_cannot_express() {
    for block in [
        "text \"one\"\n  text \"two\"\n  text \"three\"",
        "text \"big\" size=20",
        "text \"one\"\n  text \"two\" size=12",
        "text \"one\"\n  text \"two\" muted",
        "text \"one\"\n  text \"two\" color=#c00",
        "text \"one\"\n  text \"two\" align=left",
        "text \"one\" at (80,30)",
    ] {
        let src = format!("box a at (20,20) size (160,90) {{\n  {block}\n}}\n");
        assert!(lint("block-reducible", &src).is_empty(), "{block}");
    }
    assert!(
        lint(
            "block-reducible",
            "box a \"label\" at (20,20) size (160,60) sublabel=\"sub\"\n"
        )
        .is_empty()
    );
}

#[test]
fn allow_on_the_box_suppresses_it() {
    let s = scene("box a at (20,20) size (160,60) allow=block-reducible {\n  text \"one\"\n}\n");
    let d: Vec<_> = kashika_check::check(&s)
        .into_iter()
        .filter(|d| d.code == "block-reducible")
        .collect();
    assert_eq!(d.len(), 1);
    assert!(d[0].suppressed);
}
