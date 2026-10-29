mod common;

use common::*;

#[test]
fn short_step_between_vertical_runs_suggests_the_level_anchors() {
    let d = lint(
        "jog",
        "box a at (480,40) size (100,60)\nbox b at (505,300) size (60,60)\nline l a.bottom -> v 20 -> hv b.top@0.35\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "segment 2 of l jogs 4px in x between two vertical runs, from (530,120) to (526,120)"
    );
    assert_eq!(
        d[0].help,
        vec![
            "end at `b.top@0.42` to keep x=530".to_string(),
            "start at `a.bottom@0.46` to keep x=526".to_string(),
            "or move the run after the step to x=530 or the run before it to x=526, so both vertical runs share x".to_string(),
        ]
    );
    assert_eq!(d[0].elements[0].segment, Some(2));
    assert_eq!(d[0].facts["step"], 4.0);
}

#[test]
fn mid_path_step_between_horizontal_runs() {
    let d = lint(
        "jog",
        "line l (20,100) -> h 100 -> v -3 -> h 100 -> v 200\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "segment 2 of l jogs 3px in y between two horizontal runs, from (120,100) to (120,97)"
    );
    assert_eq!(
        d[0].help,
        vec![
            "move the run after the step to y=100 or the run before it to y=97, so both horizontal runs share y"
                .to_string()
        ]
    );
}

#[test]
fn rounded_and_ortho_lines_still_jog() {
    for attrs in ["radius=8", "ortho"] {
        let body = format!("line l (20,100) -> h 100 -> v 4 -> h 100 {attrs}\n");
        assert_eq!(lint("jog", &body).len(), 1, "{attrs}");
    }
}

#[test]
fn u_turns_long_steps_curves_and_straight_runs_are_fine() {
    for body in [
        "line l (20,100) -> h 100 -> v 4 -> h -100\n",
        "line l (20,100) -> h 100 -> v 6 -> h 100\n",
        "line l (20,100) -> h 100 -> curve left 2 (120,104) -> h 100\n",
        "line l (20,100) -> h 100 -> v 0 -> h 100\n",
        "line l (20,100) -> h 100 -> h 100 -> v 200\n",
        "line l (20,100) -> v 4 -> h 100\n",
    ] {
        assert!(lint("jog", body).is_empty(), "{body}");
    }
}

#[test]
fn threshold_follows_clearance() {
    let src =
        "diagram \"t\" size (800,600) clearance=12\nline l (20,100) -> h 100 -> v 10 -> h 100\n";
    let d: Vec<_> = kashika_check::check(&scene_src(src))
        .into_iter()
        .filter(|d| d.code == "jog")
        .collect();
    assert_eq!(d.len(), 1, "{d:#?}");
}

#[test]
fn allow_suppresses() {
    let d = lint(
        "jog",
        "line l (20,100) -> h 100 -> v 4 -> h 100 allow=jog reason=\"bus offset\"\n",
    );
    assert_eq!(d.len(), 1);
    assert!(d[0].suppressed);
}
