use kashika_geom::{
    FLATTEN_TOLERANCE, MAX_FLATTEN_STEPS, PathSeg, Pt, Rect, clip_to_rect, flatten, length,
    point_at, round_corners, round_corners_tagged,
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
    let t = q.trim(0.0, 10.0);
    assert!(matches!(t, PathSeg::Quad { .. }));
    assert_eq!(t.from(), q.from());
    assert!((q.length() - t.length() - 10.0).abs() < 0.5);
    let on_curve = (0..=1000)
        .map(|k| q.eval(k as f64 / 1000.0).dist(t.to()))
        .fold(f64::INFINITY, f64::min);
    assert!(on_curve < 1.0, "{on_curve}");

    let t = q.trim(10.0, 0.0);
    assert_eq!(t.to(), q.to());
    assert!((q.length() - t.length() - 10.0).abs() < 0.5);
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
