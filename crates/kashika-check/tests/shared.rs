mod common;

use common::*;

#[test]
fn collinear_overlap_is_reported_with_the_nearest_free_lane() {
    let d = lint(
        "shared-segment",
        "box s at (40,80) size (60,40)\nline a s.right -> h 200\nline b s.right -> h 100 -> v 100\n",
    );
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(
        d[0].message,
        "b runs along a for 100px, from (100,100) to (200,100)"
    );
    assert_eq!(d[0].elements[0].id, "b");
    assert_eq!(d[0].elements[1].id, "a");
    assert_eq!(
        d[0].help,
        vec![
            "nearest free lane: y=74".to_string(),
            "free y-intervals across x 100..200: 0..74, 126..600".to_string(),
        ]
    );
}

#[test]
fn parallel_or_tiny_overlaps_are_not_shared() {
    assert!(lint(
        "shared-segment",
        "line a (0,100) -> h 100\nline b (0,110) -> h 100\n"
    )
    .is_empty());
    assert!(lint(
        "shared-segment",
        "line a (0,100) -> h 100\nline b (99.5,100) -> h 100\n"
    )
    .is_empty());
}
