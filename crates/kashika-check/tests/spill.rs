mod common;

use common::*;

#[test]
fn child_outside_its_group() {
    let d = lint(
        "spill",
        "group g at (0,0) size (200,100) {\n  box a at (150,40) size (80,40)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "box 'a' extends outside group 'g' (right by 30px)"
    );
    assert_eq!(d[0].facts["overflow"], serde_json::json!({"right": 30.0}));
}

#[test]
fn bottom_title_band_is_reserved() {
    let group = |y: u32| {
        format!(
            "group g \"VPC\" at (0,0) size (200,100) title=bottom {{\n  box a at (10,{y}) size (40,40)\n}}\n"
        )
    };
    let d = lint("spill", &group(50));
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "box 'a' intrudes on the title band of group 'g'"
    );
    assert_eq!(d[0].help[0], "keep children above local y=74.5");
    assert!(lint("spill", &group(0)).is_empty());
}

#[test]
fn title_band_is_reserved() {
    let d = lint(
        "spill",
        "group g \"VPC\" at (0,0) size (200,100) {\n  box a at (10,10) size (40,40)\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "box 'a' intrudes on the title band of group 'g'"
    );
    assert!(
        lint(
            "spill",
            "group g \"VPC\" at (0,0) size (200,100) {\n  box a at (10,30) size (40,40)\n}\n"
        )
        .is_empty()
    );
    assert!(
        lint(
            "spill",
            "group g at (0,0) size (200,100) {\n  box a at (0,0) size (200,100)\n}\n"
        )
        .is_empty()
    );
}

#[test]
fn lines_and_texts_inside_groups_spill_too() {
    let d = lint(
        "spill",
        "group g at (0,0) size (200,100) {\n  line l (10,50) -> h 250\n}\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "line l extends outside group 'g' (right by 60px)"
    );
    assert_eq!(
        lint(
            "spill",
            "group g at (0,0) size (200,100) {\n  text \"far away\" at (195,50)\n}\n"
        )
        .len(),
        1
    );
}
