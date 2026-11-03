use kashika_geom::{
    FLATTEN_TOLERANCE, MAX_FLATTEN_STEPS, PathSeg, Pt, Rect, clip_to_rect, dist_point_seg, flatten,
    length, point_at, round_corners, round_corners_tagged,
};

fn line(a: (f64, f64), b: (f64, f64)) -> PathSeg {
    PathSeg::Line {
        from: Pt::new(a.0, a.1),
        to: Pt::new(b.0, b.1),
    }
}

#[test]
fn rect_helpers() {
    let r = Rect::new(10.0, 20.0, 100.0, 60.0);
    assert_eq!(r.center(), Pt::new(60.0, 50.0));
    assert_eq!((r.right(), r.bottom()), (110.0, 80.0));
}

#[test]
fn length_and_point_at_follow_arc_length() {
    let path = [
        line((0.0, 0.0), (100.0, 0.0)),
        line((100.0, 0.0), (100.0, 100.0)),
    ];
    assert!((length(&path) - 200.0).abs() < 1e-9);
    assert_eq!(point_at(&path, 0.75), Pt::new(100.0, 50.0));
}

#[test]
fn quad_flattening_stays_within_tolerance() {
    let q = PathSeg::Quad {
        from: Pt::new(0.0, 0.0),
        ctrl: Pt::new(50.0, -100.0),
        to: Pt::new(100.0, 0.0),
    };
    let pts = flatten(&[q]);
    for w in pts.windows(2) {
        let mid = w[0].lerp(w[1], 0.5);
        let t = mid.x / 100.0;
        let exact = q.eval(t);
        assert!((exact.y - mid.y).abs() <= FLATTEN_TOLERANCE + 1e-9);
    }
}

#[test]
fn axis_alignment() {
    assert!(line((0.0, 0.0), (0.0, 50.0)).is_axis_aligned());
    assert!(!line((0.0, 0.0), (10.0, 50.0)).is_axis_aligned());
}

#[test]
fn rounds_polyline_corners_and_keeps_curves() {
    let path = [
        line((0.0, 0.0), (100.0, 0.0)),
        line((100.0, 0.0), (100.0, 100.0)),
    ];
    let r = round_corners(&path, 10.0);
    assert_eq!(
        r,
        vec![
            line((0.0, 0.0), (90.0, 0.0)),
            PathSeg::Quad {
                from: Pt::new(90.0, 0.0),
                ctrl: Pt::new(100.0, 0.0),
                to: Pt::new(100.0, 10.0)
            },
            line((100.0, 10.0), (100.0, 100.0)),
        ]
    );
}

#[test]
fn radius_is_clamped_to_half_the_shorter_segment() {
    let path = [
        line((0.0, 0.0), (10.0, 0.0)),
        line((10.0, 0.0), (10.0, 100.0)),
    ];
    let r = round_corners(&path, 50.0);
    assert_eq!(r[0], line((0.0, 0.0), (5.0, 0.0)));
}

#[test]
fn directions_at_ends() {
    let q = PathSeg::Quad {
        from: Pt::new(0.0, 0.0),
        ctrl: Pt::new(0.0, -10.0),
        to: Pt::new(10.0, -10.0),
    };
    assert_eq!(q.start_dir(), Pt::new(0.0, -1.0));
    assert_eq!(q.end_dir(), Pt::new(1.0, 0.0));
}

#[test]
fn flattening_huge_curves_is_bounded() {
    let c = PathSeg::Cubic {
        from: Pt::new(0.0, 0.0),
        c1: Pt::new(1e300, 0.0),
        c2: Pt::new(0.0, 1e300),
        to: Pt::new(10.0, 10.0),
    };
    assert!(flatten(&[c]).len() <= MAX_FLATTEN_STEPS + 1);
}

#[test]
fn trim_follows_the_curve() {
    let q = PathSeg::Quad {
        from: Pt::new(0.0, 0.0),
        ctrl: Pt::new(50.0, -100.0),
        to: Pt::new(100.0, 0.0),
    };
    // `trim` finds the cut by arc length on the polyline flattened to FLATTEN_TOLERANCE,
    // and `length` measures each curve on its own flattening, so the removed length is
    // off by about 0.17 px here (0.16 against the exact arc length): 0.05 is out of reach.
    let removed_slack = 0.2;
    let t = q.trim(0.0, 10.0);
    assert!(matches!(t, PathSeg::Quad { .. }));
    assert_eq!(t.from(), q.from());
    assert!((q.length() - t.length() - 10.0).abs() < removed_slack);
    let on_curve = (0..=100_000)
        .map(|k| q.eval(k as f64 / 100_000.0).dist(t.to()))
        .fold(f64::INFINITY, f64::min);
    assert!(on_curve < 0.01, "{on_curve}");

    let t = q.trim(10.0, 0.0);
    assert_eq!(t.to(), q.to());
    assert!((q.length() - t.length() - 10.0).abs() < removed_slack);
}

#[test]
fn trim_splits_overlapping_ends_in_proportion() {
    let t = line((0.0, 0.0), (15.0, 0.0)).trim(10.0, 10.0);
    assert_eq!(t, line((7.5, 0.0), (7.5, 0.0)));
    let t = line((0.0, 0.0), (100.0, 0.0)).trim(10.0, 12.0);
    assert_eq!(t, line((10.0, 0.0), (88.0, 0.0)));
}

#[test]
fn rounding_keeps_segment_tags() {
    let path = [
        (line((0.0, 0.0), (100.0, 0.0)), 0usize),
        (line((100.0, 0.0), (100.0, 100.0)), 1),
        (line((100.0, 100.0), (200.0, 100.0)), 1),
    ];
    let out = round_corners_tagged(&path, 10.0);
    let tags: Vec<usize> = out.iter().map(|(_, t)| *t).collect();
    assert_eq!(tags, vec![0, 0, 1, 1, 1]);
    let plain: Vec<PathSeg> = out.into_iter().map(|(s, _)| s).collect();
    let untagged: Vec<PathSeg> = path.iter().map(|(s, _)| *s).collect();
    assert_eq!(plain, round_corners(&untagged, 10.0));
}

#[test]
fn clip_to_rect_keeps_the_part_inside() {
    let tri = [Pt::new(0.0, 0.0), Pt::new(20.0, 0.0), Pt::new(0.0, 20.0)];
    let r = Rect::new(5.0, 0.0, 10.0, 10.0);
    let c = clip_to_rect(&tri, r);
    assert_eq!(Rect::bounding(c.iter().copied()), r);
    assert!(c.iter().all(|p| p.x >= 5.0 && p.x <= 15.0 && p.y <= 10.0));
    assert!(clip_to_rect(&tri, Rect::new(30.0, 30.0, 5.0, 5.0)).is_empty());
    assert_eq!(
        clip_to_rect(&tri, Rect::new(-1.0, -1.0, 30.0, 30.0)).len(),
        3
    );
}

fn quad(a: (f64, f64), c: (f64, f64), b: (f64, f64)) -> PathSeg {
    PathSeg::Quad {
        from: Pt::new(a.0, a.1),
        ctrl: Pt::new(c.0, c.1),
        to: Pt::new(b.0, b.1),
    }
}

#[test]
fn rect_union_spans_disjoint_rects() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(20.0, 30.0, 5.0, 5.0);
    assert_eq!(a.union(b), Rect::new(0.0, 0.0, 25.0, 35.0));
    assert_eq!(b.union(a), a.union(b));
    assert_eq!(a.union(Rect::new(2.0, 2.0, 3.0, 3.0)), a);
}

#[test]
fn rect_intersect_needs_more_than_tolerance_both_ways() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(a.intersect(Rect::new(9.5, 0.0, 10.0, 10.0)), None);
    assert_eq!(
        a.intersect(Rect::new(9.25, 2.0, 10.0, 10.0)),
        Some(Rect::new(9.25, 2.0, 0.75, 8.0))
    );
    assert_eq!(a.intersect(Rect::new(0.0, 9.5, 10.0, 10.0)), None);
    assert_eq!(
        a.intersect(Rect::new(2.0, 3.0, 4.0, 5.0)),
        Some(Rect::new(2.0, 3.0, 4.0, 5.0))
    );
    assert_eq!(a.intersect(Rect::new(50.0, 50.0, 5.0, 5.0)), None);
}

#[test]
fn rect_contains_rect_allows_tolerance_overhang() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert!(a.contains_rect(Rect::new(-0.5, -0.5, 11.0, 11.0)));
    assert!(!a.contains_rect(Rect::new(-0.75, 0.0, 5.0, 5.0)));
    assert!(!a.contains_rect(Rect::new(0.0, 0.0, 5.0, 10.75)));
    assert!(a.contains_rect(Rect::new(5.0, 5.0, 0.0, 0.0)));
}

#[test]
fn rect_from_points_and_bounding_degenerate_inputs() {
    let p = Pt::new(3.0, -4.0);
    assert_eq!(Rect::from_points(p, p), Rect::new(3.0, -4.0, 0.0, 0.0));
    assert_eq!(
        Rect::from_points(Pt::new(5.0, 0.0), Pt::new(1.0, 8.0)),
        Rect::new(1.0, 0.0, 4.0, 8.0)
    );
    assert_eq!(Rect::bounding([p]), Rect::new(3.0, -4.0, 0.0, 0.0));
    assert_eq!(
        Rect::bounding([
            Pt::new(1.0, 1.0),
            Pt::new(-2.0, 5.0),
            Pt::new(4.0, -3.0),
            Pt::new(0.0, 0.0),
        ]),
        Rect::new(-2.0, -3.0, 6.0, 8.0)
    );
}

#[test]
fn split_cubic_at_a_quarter() {
    let c = PathSeg::Cubic {
        from: Pt::new(0.0, 0.0),
        c1: Pt::new(0.0, 100.0),
        c2: Pt::new(100.0, 100.0),
        to: Pt::new(100.0, 0.0),
    };
    // de Casteljau at t = 1/4: (0,25) (25,100) (100,75) -> (6.25,43.75) (43.75,93.75)
    // -> (15.625,56.25), which is B(1/4).
    let (a, b) = c.split(0.25);
    assert_eq!(
        a,
        PathSeg::Cubic {
            from: Pt::new(0.0, 0.0),
            c1: Pt::new(0.0, 25.0),
            c2: Pt::new(6.25, 43.75),
            to: Pt::new(15.625, 56.25),
        }
    );
    assert_eq!(
        b,
        PathSeg::Cubic {
            from: Pt::new(15.625, 56.25),
            c1: Pt::new(43.75, 93.75),
            c2: Pt::new(100.0, 75.0),
            to: Pt::new(100.0, 0.0),
        }
    );
    assert_eq!(c.eval(0.25), Pt::new(15.625, 56.25));
}

#[test]
fn split_line_and_quad() {
    let (a, b) = line((0.0, 0.0), (8.0, 4.0)).split(0.25);
    assert_eq!(
        (a, b),
        (line((0.0, 0.0), (2.0, 1.0)), line((2.0, 1.0), (8.0, 4.0)))
    );
    let (a, b) = quad((0.0, 0.0), (4.0, 8.0), (8.0, 0.0)).split(0.5);
    assert_eq!(
        (a, b),
        (
            quad((0.0, 0.0), (2.0, 4.0), (4.0, 4.0)),
            quad((4.0, 4.0), (6.0, 4.0), (8.0, 0.0))
        )
    );
}

#[test]
fn directions_of_degenerate_segments() {
    let z = line((3.0, 3.0), (3.0, 3.0));
    assert_eq!(z.start_dir(), Pt::new(0.0, 0.0));
    assert_eq!(z.end_dir(), Pt::new(0.0, 0.0));
    let c = PathSeg::Cubic {
        from: Pt::new(0.0, 0.0),
        c1: Pt::new(0.0, 0.0),
        c2: Pt::new(10.0, 0.0),
        to: Pt::new(10.0, 0.0),
    };
    assert_eq!(c.start_dir(), Pt::new(1.0, 0.0));
    assert_eq!(c.end_dir(), Pt::new(1.0, 0.0));
    let q = quad((0.0, 0.0), (0.0, 0.0), (0.0, 5.0));
    assert_eq!(q.start_dir(), Pt::new(0.0, 1.0));
}

#[test]
fn point_at_degenerate_paths_and_out_of_range_t() {
    assert_eq!(point_at(&[], 0.5), Pt::new(0.0, 0.0));
    let z = [line((5.0, 7.0), (5.0, 7.0))];
    assert_eq!(point_at(&z, 0.0), Pt::new(5.0, 7.0));
    assert_eq!(point_at(&z, 0.5), Pt::new(5.0, 7.0));
    let path = [line((0.0, 0.0), (100.0, 0.0))];
    assert_eq!(point_at(&path, -0.5), Pt::new(0.0, 0.0));
    assert_eq!(point_at(&path, 1.5), Pt::new(100.0, 0.0));
}

#[test]
fn cubic_flattening_stays_within_tolerance() {
    let c = PathSeg::Cubic {
        from: Pt::new(0.0, 0.0),
        c1: Pt::new(30.0, 200.0),
        c2: Pt::new(170.0, -120.0),
        to: Pt::new(200.0, 50.0),
    };
    let pts = flatten(&[c]);
    assert!(pts.len() > 2);
    for k in 0..=50 {
        let p = c.eval(k as f64 / 50.0);
        let d = pts
            .windows(2)
            .map(|w| dist_point_seg(p, w[0], w[1]))
            .fold(f64::INFINITY, f64::min);
        assert!(d <= FLATTEN_TOLERANCE, "sample {k}: {d}");
    }
}

#[test]
fn rounding_a_collinear_corner_keeps_a_straight_quad() {
    let path = [
        line((0.0, 0.0), (50.0, 0.0)),
        line((50.0, 0.0), (100.0, 0.0)),
    ];
    assert_eq!(
        round_corners(&path, 10.0),
        vec![
            line((0.0, 0.0), (40.0, 0.0)),
            quad((40.0, 0.0), (50.0, 0.0), (60.0, 0.0)),
            line((60.0, 0.0), (100.0, 0.0)),
        ]
    );
}

#[test]
fn rounding_a_reversal_folds_back_on_itself() {
    let path = [
        line((0.0, 0.0), (50.0, 0.0)),
        line((50.0, 0.0), (20.0, 0.0)),
    ];
    assert_eq!(
        round_corners(&path, 10.0),
        vec![
            line((0.0, 0.0), (40.0, 0.0)),
            quad((40.0, 0.0), (50.0, 0.0), (40.0, 0.0)),
            line((40.0, 0.0), (20.0, 0.0)),
        ]
    );
}

#[test]
fn a_tiny_radius_leaves_sharp_corners() {
    let path = [
        line((0.0, 0.0), (100.0, 0.0)),
        line((100.0, 0.0), (100.0, 100.0)),
    ];
    assert_eq!(round_corners(&path, 1e-10), path.to_vec());
    assert_eq!(round_corners(&path, 0.0), path.to_vec());
    let with_dup = [
        line((0.0, 0.0), (100.0, 0.0)),
        line((100.0, 0.0), (100.0, 0.0)),
        line((100.0, 0.0), (100.0, 100.0)),
    ];
    assert_eq!(round_corners(&with_dup, 10.0), path.to_vec());
}

#[test]
fn clip_to_rect_inside_and_touching() {
    let tri = [Pt::new(2.0, 2.0), Pt::new(8.0, 2.0), Pt::new(5.0, 8.0)];
    let r = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(clip_to_rect(&tri, r), tri.to_vec());
    let below = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(5.0, -5.0)];
    assert_eq!(
        clip_to_rect(&below, r),
        vec![
            Pt::new(0.0, 0.0),
            Pt::new(0.0, 0.0),
            Pt::new(10.0, 0.0),
            Pt::new(10.0, 0.0)
        ]
    );
    let corner = [
        Pt::new(10.0, 10.0),
        Pt::new(20.0, 10.0),
        Pt::new(20.0, 20.0),
    ];
    assert_eq!(clip_to_rect(&corner, r), vec![Pt::new(10.0, 10.0); 3]);
}
