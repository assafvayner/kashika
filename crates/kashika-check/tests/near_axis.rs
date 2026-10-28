mod common;

use common::*;

#[test]
fn slight_slant_suggests_the_level_anchor() {
    let d = lint(
        "near-axis",
        "box users at (20,100) size (100,60)\nbox cdn at (400,105) size (100,60)\nline l users.right -> cdn.left\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "segment 1 of l is 5px (1.02°) off horizontal, from (120,130) to (400,135)"
    );
    assert_eq!(
        d[0].help,
        vec![
            "end at `cdn.left@0.42` to keep y=130".to_string(),
            "start at `users.right@0.58` to keep y=135".to_string(),
            "or move one end 5px in y so both ends share it, or use `hv`/`vh` (or an `x`/`y` target) for a deliberate elbow".to_string(),
        ]
    );
}

#[test]
fn vertical_slant_and_offset_anchor() {
    let d = lint(
        "near-axis",
        "box a at (100,20) size (90,40)\nbox b at (103,300) size (90,40)\nline l a.bottom -> b.top\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert!(d[0].message.contains("3px"), "{}", d[0].message);
    assert!(d[0].message.contains("off vertical"), "{}", d[0].message);
    assert_eq!(d[0].help[0], "end at `b.top+(-3,0)` to keep x=145");
}

#[test]
fn axis_aligned_steep_ortho_and_curves_are_fine() {
    for body in [
        "line l (0,100) -> h 300\n",
        "line l (0,100) -> (300,100.4)\n",
        "line l (0,100) -> (300,130)\n",
        "line l (0,100) -> (300,105) ortho\n",
        "line l (0,100) -> curve left 2 (300,105)\n",
    ] {
        let (s, _) = kashika_model::compile(&format!("{HEAD}{body}"));
        let d: Vec<_> = kashika_check::check(&s)
            .into_iter()
            .filter(|d| d.code == "near-axis")
            .collect();
        assert!(d.is_empty(), "{body}: {d:#?}");
    }
}

#[test]
fn near_axis_tails_are_not_diagonal_tails() {
    let body = "line l (0,0) -> h 100 -> v 50 -> (300,55)\n";
    assert_eq!(lint("near-axis", body).len(), 1);
    assert!(lint("diagonal-tail", body).is_empty());
}

#[test]
fn offset_anchor_when_t_does_not_land_exactly() {
    let d = lint(
        "near-axis",
        "box a at (20,100) size (100,600)\nbox b at (400,0) size (100,810)\nline l a.right -> b.left\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].help[0], "end at `b.left+(0,-5)` to keep y=400");
}
