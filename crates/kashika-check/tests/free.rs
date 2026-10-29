mod common;

use common::*;
use kashika_check::ctx::Ctx;
use kashika_check::free::{
    Axis, DeferredKind, corridors, free_around, free_intervals, resolve, with_deferred, with_free,
};
use kashika_diag::{Diagnostic, Span};
use kashika_geom::{Pt, Rect};

#[test]
fn free_intervals_avoid_boxes_and_lines_with_clearance() {
    let s = scene("box a at (100,110) size (80,60)\nline l (0,300) -> h 800\n");
    let ctx = Ctx::new(&s);
    assert_eq!(
        free_intervals(&ctx, Axis::Y, (50.0, 500.0), (0.0, 600.0), &[]),
        vec![(0.0, 104.0), (176.0, 294.0), (306.0, 600.0)]
    );
    assert_eq!(
        free_intervals(&ctx, Axis::Y, (50.0, 500.0), (0.0, 600.0), &["l"]),
        vec![(0.0, 104.0), (176.0, 600.0)]
    );
    assert_eq!(
        free_intervals(&ctx, Axis::Y, (200.0, 500.0), (0.0, 600.0), &["l"]),
        vec![(0.0, 600.0)]
    );
}

#[test]
fn free_intervals_avoid_group_borders_but_not_their_interior() {
    let s = scene("group g at (100,100) size (200,200) {\n}\n");
    let ctx = Ctx::new(&s);
    assert_eq!(
        free_intervals(&ctx, Axis::X, (0.0, 600.0), (0.0, 800.0), &[]),
        vec![(0.0, 94.0), (106.0, 294.0), (306.0, 800.0)]
    );
    assert_eq!(
        free_intervals(&ctx, Axis::X, (400.0, 600.0), (0.0, 800.0), &[]),
        vec![(0.0, 800.0)]
    );
}

#[test]
fn free_intervals_drop_gaps_that_round_to_nothing() {
    let s = scene("box a at (0,0) size (100,100)\nbox b at (0,112.004) size (100,50)\n");
    let ctx = Ctx::new(&s);
    assert_eq!(
        free_intervals(&ctx, Axis::Y, (10.0, 90.0), (0.0, 600.0), &[]),
        vec![(168.0, 600.0)]
    );
}

#[test]
fn free_intervals_avoid_texts_labels_and_junctions() {
    let free_in = |body: &str, band: (f64, f64), skip: &[&str]| {
        let s = scene(body);
        free_intervals(&Ctx::new(&s), Axis::Y, band, (0.0, 600.0), skip)
    };
    let free = |body: &str, skip: &[&str]| free_in(body, (0.0, 800.0), skip);
    let text = free("text \"note\" at (400,300)\n", &[]);
    assert_eq!(text.len(), 2, "{text:?}");
    assert!(text[0].1 < 294.0 && text[1].0 > 306.0, "{text:?}");
    let junction = free("junction j at (400,300)\n", &[]);
    assert_eq!(junction, vec![(0.0, 294.0), (306.0, 600.0)]);
    let labeled = "line l (400,250) -> v 100 label=\"a wide label\"\n";
    let label = free_in(labeled, (410.0, 800.0), &[]);
    assert_eq!(label.len(), 2, "{label:?}");
    assert!(label[0].1 < 294.0 && label[1].0 > 306.0, "{label:?}");
    assert_eq!(free_in(labeled, (410.0, 800.0), &["l"]), vec![(0.0, 600.0)]);
    assert_eq!(
        free_in("line l (400,250) -> v 100\n", (410.0, 800.0), &[]),
        vec![(0.0, 600.0)]
    );
}

#[test]
fn free_intervals_avoid_the_label_under_a_below_shape() {
    let mut s = scene("box b \"Below\" at (380,200) size (80,40)\n");
    s.shapes[0].text_below = true;
    let label = s.shapes[0].text[0].rect;
    for item in &mut s.shapes[0].text {
        item.rect.y += 60.0;
    }
    let ctx = Ctx::new(&s);
    let free = free_intervals(&ctx, Axis::Y, (0.0, 800.0), (0.0, 600.0), &[]);
    assert_eq!(free.len(), 3, "{free:?}");
    assert!(free[2].0 >= label.bottom() + 60.0, "{free:?}");
    let skipped = free_intervals(&ctx, Axis::Y, (0.0, 800.0), (0.0, 600.0), &["b"]);
    assert_eq!(skipped, vec![(0.0, 600.0)]);
}

#[test]
fn free_around_a_horizontal_piece_and_its_nearest_lane() {
    let s = scene("box a at (100,110) size (80,60)\n");
    let ctx = Ctx::new(&s);
    let f = free_around(&ctx, Pt::new(50.0, 150.0), Pt::new(300.0, 150.0), &[]).unwrap();
    assert_eq!(f.axis, Axis::Y);
    assert_eq!(
        f.describe(),
        "free y-intervals across x 50..300: 0..104, 176..600"
    );
    assert_eq!(f.nearest(), Some(176.0));
    assert_eq!(
        f.fact(),
        serde_json::json!({"axis": "y", "band": [50.0, 300.0], "intervals": [[0.0, 104.0], [176.0, 600.0]]})
    );
    assert!(free_around(&ctx, Pt::new(0.0, 0.0), Pt::new(10.0, 10.0), &[]).is_none());
}

#[test]
fn free_around_a_vertical_piece_spans_the_canvas_width() {
    let s = scene("box a at (100,110) size (80,60)\n");
    let ctx = Ctx::new(&s);
    let f = free_around(&ctx, Pt::new(150.0, 0.0), Pt::new(150.0, 400.0), &[]).unwrap();
    assert_eq!(f.axis, Axis::X);
    assert_eq!(f.intervals, vec![(0.0, 94.0), (186.0, 800.0)]);
    assert_eq!(f.nearest(), Some(186.0));
}

#[test]
fn with_free_adds_a_fact_and_help_for_straight_pieces() {
    let s = scene("box a at (100,110) size (80,60)\nline l (50,150) -> h 250 -> v 100\n");
    let ctx = Ctx::new(&s);
    let l = ctx.line("l").unwrap();
    let f = with_free(
        Diagnostic::warn("through", Span::default(), "x"),
        l,
        0,
        &["l"],
    );
    assert!(f.diag.help.is_empty() && f.deferred.len() == 1);
    let d = f.resolved(&ctx);
    assert_eq!(
        d.help,
        ["segment 1 runs at y=150; free y-intervals across x 50..300: 0..104, 176..600"]
    );
    assert!(d.facts.contains_key("free"));
}

#[test]
fn corridors_cover_both_axes() {
    let s = scene("box a at (100,100) size (100,100)\n");
    let (h, v) = corridors(&s, Rect::new(0.0, 0.0, 400.0, 400.0));
    assert_eq!(h, vec![(0.0, 94.0), (206.0, 400.0)]);
    assert_eq!(v, vec![(0.0, 94.0), (206.0, 400.0)]);
}

#[test]
fn deferred_help_resolves_to_what_check_reports() {
    let s = scene(
        "box a at (100,100) size (80,60)\nbox b at (400,100) size (80,60)\n\
         line lx (0,130) -> h 600 allow=cross reason=\"bus\"\nline ly (300,0) -> v 400\n\
         line p (0,300) -> h 400\nline q (100,300) -> h 400\n\
         line n (0,163) -> h 150\n",
    );
    let eager = kashika_check::check(&s);
    let ctx = Ctx::new(&s);
    let lazy = kashika_check::check_bounded(&ctx, usize::MAX, &|_| true);
    assert_eq!(lazy.diags, kashika_check::check_deferred(&s));
    let pending = || {
        lazy.diags
            .iter()
            .zip(&lazy.deferred)
            .filter(|(_, x)| !x.is_empty())
    };
    let codes: Vec<&str> = pending().map(|(d, _)| d.code).collect();
    for code in ["cross", "through", "shared-segment", "near"] {
        assert!(codes.contains(&code), "{code} not deferred: {codes:?}");
    }
    assert!(
        pending().any(|(d, _)| d.suppressed),
        "no suppressed finding with deferred help"
    );
    assert_ne!(lazy.diags, eager);
    let resolved: Vec<Diagnostic> = lazy
        .diags
        .into_iter()
        .zip(lazy.deferred)
        .map(|(mut d, x)| {
            resolve(&ctx, &mut d, x);
            d
        })
        .collect();
    assert_eq!(resolved, eager);
}

#[test]
fn a_lane_with_nowhere_free_lists_no_lane() {
    let s = scene("box wall at (0,0) size (800,600)\n");
    let ctx = Ctx::new(&s);
    let d = with_deferred(
        Diagnostic::warn("shared-segment", Span::default(), "x"),
        DeferredKind::Lane,
        Pt::new(100.0, 300.0),
        Pt::new(200.0, 300.0),
        &[],
    )
    .resolved(&ctx);
    assert_eq!(d.help, ["free y-intervals across x 100..200: none"]);
    assert!(!d.facts.contains_key("lane"));
    assert_eq!(d.facts["free"]["intervals"], serde_json::json!([]));
}

#[test]
fn two_deferred_entries_keep_their_place_in_the_help() {
    let s = scene("box a at (100,110) size (80,60)\nline l (50,150) -> h 250 -> v 100\n");
    let ctx = Ctx::new(&s);
    let l = ctx.line("l").unwrap();
    let d = Diagnostic::warn("through", Span::default(), "x").with_help("first");
    let d = with_free(d, l, 0, &["l"]).with_help("second");
    let f = with_deferred(
        d,
        DeferredKind::Lane,
        Pt::new(50.0, 150.0),
        Pt::new(300.0, 150.0),
        &["l"],
    )
    .with_help("third");
    assert_eq!(f.deferred.len(), 2);
    let d = f.resolved(&ctx);
    assert_eq!(
        d.help,
        [
            "first",
            "segment 1 runs at y=150; free y-intervals across x 50..300: 0..104, 176..600",
            "second",
            "nearest free lane: y=176",
            "free y-intervals across x 50..300: 0..104, 176..600",
            "third",
        ]
    );
    assert_eq!(
        d.facts["lane"],
        serde_json::json!({"axis": "y", "at": 176.0})
    );
}
