use kashika_geom::{
    boundary_nearest, dist_point_seg, inside_runs, pieces, point_in_polygon, polygons_overlap,
    rect_polygon, seg_hit, seg_seg_dist, strictly_inside, Hit, PathSeg, Pt, Rect,
};

fn p(x: f64, y: f64) -> Pt {
    Pt::new(x, y)
}

#[test]
fn crossing_segments_meet_at_one_point() {
    assert_eq!(
        seg_hit(p(0.0, 0.0), p(10.0, 10.0), p(0.0, 10.0), p(10.0, 0.0), 0.5),
        Some(Hit::Point(p(5.0, 5.0)))
    );
}

#[test]
fn touching_within_tolerance_counts() {
    assert_eq!(
        seg_hit(p(0.0, 0.0), p(10.0, 0.0), p(5.0, 0.3), p(5.0, 10.0), 0.5),
        Some(Hit::Point(p(5.0, 0.3)))
    );
    assert_eq!(
        seg_hit(p(0.0, 0.0), p(10.0, 0.0), p(5.0, 1.0), p(5.0, 10.0), 0.5),
        None
    );
}

#[test]
fn collinear_overlap_and_parallel_miss() {
    assert_eq!(
        seg_hit(p(0.0, 0.0), p(10.0, 0.0), p(5.0, 0.0), p(20.0, 0.0), 0.5),
        Some(Hit::Overlap(p(5.0, 0.0), p(10.0, 0.0)))
    );
    assert_eq!(
        seg_hit(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 3.0), p(10.0, 3.0), 0.5),
        None
    );
}

#[test]
fn distances() {
    assert_eq!(dist_point_seg(p(5.0, 5.0), p(0.0, 0.0), p(10.0, 0.0)), 5.0);
    assert_eq!(
        seg_seg_dist(p(0.0, 0.0), p(10.0, 0.0), p(0.0, 4.0), p(10.0, 4.0)),
        4.0
    );
    assert_eq!(
        seg_seg_dist(p(0.0, 0.0), p(10.0, 10.0), p(0.0, 10.0), p(10.0, 0.0)),
        0.0
    );
}

#[test]
fn boundary_distance_and_inside() {
    let sq = rect_polygon(Rect::new(0.0, 0.0, 10.0, 10.0));
    assert_eq!(boundary_nearest(p(5.0, 3.0), &sq), (3.0, p(5.0, 0.0)));
    assert!(strictly_inside(p(5.0, 5.0), &sq));
    assert!(!strictly_inside(p(5.0, 0.2), &sq));
    assert!(!point_in_polygon(p(15.0, 5.0), &sq));
}

#[test]
fn inside_runs_clip_to_the_interior() {
    let sq = rect_polygon(Rect::new(10.0, 10.0, 20.0, 20.0));
    assert_eq!(
        inside_runs(p(0.0, 20.0), p(40.0, 20.0), &sq),
        vec![(p(10.0, 20.0), p(30.0, 20.0))]
    );
    assert!(inside_runs(p(0.0, 10.0), p(40.0, 10.0), &sq).is_empty());
    assert!(inside_runs(p(0.0, 0.0), p(5.0, 40.0), &sq).is_empty());
    assert_eq!(
        inside_runs(p(20.0, 20.0), p(40.0, 20.0), &sq),
        vec![(p(20.0, 20.0), p(30.0, 20.0))]
    );
}

#[test]
fn polygon_overlap_ignores_touching() {
    let a = rect_polygon(Rect::new(0.0, 0.0, 10.0, 10.0));
    assert!(!polygons_overlap(
        &a,
        &rect_polygon(Rect::new(10.0, 0.0, 10.0, 10.0))
    ));
    assert!(polygons_overlap(
        &a,
        &rect_polygon(Rect::new(5.0, 5.0, 10.0, 10.0))
    ));
    assert!(polygons_overlap(
        &a,
        &rect_polygon(Rect::new(0.0, 0.0, 10.0, 10.0))
    ));
    assert!(polygons_overlap(
        &a,
        &rect_polygon(Rect::new(2.0, 2.0, 3.0, 3.0))
    ));
}

#[test]
fn pieces_flatten_curves_and_keep_indices() {
    let path = [
        PathSeg::Line {
            from: p(0.0, 0.0),
            to: p(10.0, 0.0),
        },
        PathSeg::Quad {
            from: p(10.0, 0.0),
            ctrl: p(20.0, 0.0),
            to: p(20.0, 10.0),
        },
    ];
    let ps = pieces(&path);
    assert_eq!(ps[0], (p(0.0, 0.0), p(10.0, 0.0), 0));
    assert!(ps.len() > 2 && ps[1..].iter().all(|x| x.2 == 1));
    assert_eq!(ps.last().unwrap().1, p(20.0, 10.0));
}
