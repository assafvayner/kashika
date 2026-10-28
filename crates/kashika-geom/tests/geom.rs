use kashika_geom::{
    flatten, length, point_at, round_corners, PathSeg, Pt, Rect, FLATTEN_TOLERANCE,
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
