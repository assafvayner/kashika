mod common;

use common::*;
use kashika_check::ctx::Ctx;
use kashika_check::free::{corridors, free_around, free_intervals, with_free, Axis};
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
    let d = with_free(
        &ctx,
        Diagnostic::warn("through", Span::default(), "x"),
        l,
        0,
        &["l"],
    );
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
