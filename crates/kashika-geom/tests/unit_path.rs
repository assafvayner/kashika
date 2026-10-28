use kashika_geom::{
    PathSeg, Pt, Rect, arc_to_cubics, convex_hull, horizontal_hits, parse_unit_path, vertical_hits,
};

#[test]
fn parses_lines_and_close() {
    let sp = parse_unit_path("M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z").unwrap();
    assert_eq!(sp.len(), 1);
    assert_eq!(sp[0].len(), 4);
    assert_eq!(
        sp[0][3],
        PathSeg::Line {
            from: Pt::new(0.0, 0.5),
            to: Pt::new(0.5, 0.0)
        }
    );
}

#[test]
fn h_v_q_c_and_implicit_lineto() {
    let sp = parse_unit_path("M 0,0 1,0 V 1 H 0.5 Q 0.25 1 0 0.5 C 0 0.3 0.1 0 0 0").unwrap();
    let s = &sp[0];
    assert_eq!(s.len(), 5);
    assert_eq!(
        s[0],
        PathSeg::Line {
            from: Pt::new(0.0, 0.0),
            to: Pt::new(1.0, 0.0)
        }
    );
    assert_eq!(s[1].to(), Pt::new(1.0, 1.0));
    assert_eq!(s[2].to(), Pt::new(0.5, 1.0));
    assert!(matches!(s[3], PathSeg::Quad { .. }));
    assert!(matches!(s[4], PathSeg::Cubic { .. }));
}

#[test]
fn multiple_subpaths() {
    let sp = parse_unit_path("M 0 0 L 1 1  M 0 1 L 1 0").unwrap();
    assert_eq!(sp.len(), 2);
    assert_eq!(sp[1][0].from(), Pt::new(0.0, 1.0));
}

#[test]
fn arcs_become_cubics_on_the_requested_side() {
    let sp = parse_unit_path("M 0 0.5 A 0.5 0.5 0 0 1 1 0.5").unwrap();
    assert_eq!(sp[0].len(), 2);
    assert!(sp[0].iter().all(|s| matches!(s, PathSeg::Cubic { .. })));
    assert!(
        sp[0][0].to().dist(Pt::new(0.5, 0.0)) < 1e-9,
        "sweep=1 goes over the top on screen"
    );
    assert!(sp[0][1].to().dist(Pt::new(1.0, 0.5)) < 1e-12);
    let sp = parse_unit_path("M 0 0.5 A 0.5 0.5 0 0 0 1 0.5").unwrap();
    assert!(
        sp[0][0].to().dist(Pt::new(0.5, 1.0)) < 1e-9,
        "sweep=0 goes under"
    );
}

#[test]
fn arc_stays_close_to_the_circle() {
    let segs = arc_to_cubics(
        Pt::new(0.0, 50.0),
        50.0,
        50.0,
        0.0,
        false,
        true,
        Pt::new(100.0, 50.0),
    );
    for s in &segs {
        for k in 0..=10 {
            let p = s.eval(k as f64 / 10.0);
            assert!((p.dist(Pt::new(50.0, 50.0)) - 50.0).abs() < 0.05, "{p:?}");
        }
    }
}

#[test]
fn small_radii_are_scaled_up() {
    let segs = arc_to_cubics(
        Pt::new(0.0, 0.0),
        0.1,
        0.1,
        0.0,
        false,
        true,
        Pt::new(1.0, 0.0),
    );
    assert!(segs.last().unwrap().to().dist(Pt::new(1.0, 0.0)) < 1e-12);
    assert!(segs[0].to().dist(Pt::new(0.5, -0.5)) < 1e-9);
}

#[test]
fn rejects_bad_paths() {
    for (d, msg) in [
        ("", "empty"),
        ("L 1 1", "must start with `M`"),
        ("m 0 0", "relative"),
        ("M 0 0 X 1", "unknown path command"),
        ("M 0 0 L 1.5 0", "between 0 and 1"),
        ("M 0 0 L 1", "expects 2 numbers"),
        ("M 0 0 A 1 1 0 2 0 1 1", "flags"),
        ("M 0 0 Z 1 1", "numbers must follow a command"),
        ("M 0 0 L 0.5.5 1", "bad number"),
    ] {
        let e = parse_unit_path(d).unwrap_err();
        assert!(e.contains(msg), "{d:?}: {e}");
    }
}

#[test]
fn unit_mapping() {
    let r = Rect::new(100.0, 50.0, 200.0, 100.0);
    assert_eq!(r.map_unit(Pt::new(0.5, 0.25)), Pt::new(200.0, 75.0));
    assert_eq!(
        r.map_rect(Rect::new(0.25, 0.5, 0.5, 0.5)),
        Rect::new(150.0, 100.0, 100.0, 50.0)
    );
    let s = PathSeg::Line {
        from: Pt::new(0.0, 0.0),
        to: Pt::new(1.0, 1.0),
    }
    .map(|p| r.map_unit(p));
    assert_eq!(s.to(), Pt::new(300.0, 150.0));
}

#[test]
fn hull_drops_interior_and_duplicate_points() {
    let pts = [
        Pt::new(0.0, 0.0),
        Pt::new(1.0, 0.0),
        Pt::new(1.0, 1.0),
        Pt::new(0.0, 1.0),
        Pt::new(0.5, 0.5),
        Pt::new(1.0, 1.0),
    ];
    let h = convex_hull(&pts);
    assert_eq!(h.len(), 4);
    for p in [
        Pt::new(0.0, 0.0),
        Pt::new(1.0, 0.0),
        Pt::new(1.0, 1.0),
        Pt::new(0.0, 1.0),
    ] {
        assert!(h.contains(&p), "{p:?} missing from {h:?}");
    }
}

#[test]
fn hull_of_collinear_points_is_its_endpoints() {
    let h = convex_hull(&[Pt::new(0.0, 0.0), Pt::new(1.0, 1.0), Pt::new(2.0, 2.0)]);
    assert_eq!(h, vec![Pt::new(0.0, 0.0), Pt::new(2.0, 2.0)]);
}

#[test]
fn axis_hits_on_a_diamond() {
    let d = [
        Pt::new(60.0, 0.0),
        Pt::new(120.0, 40.0),
        Pt::new(60.0, 80.0),
        Pt::new(0.0, 40.0),
    ];
    let mut ys = vertical_hits(&d, 30.0);
    ys.sort_by(f64::total_cmp);
    assert_eq!(ys, vec![20.0, 60.0]);
    let mut xs = horizontal_hits(&d, 20.0);
    xs.sort_by(f64::total_cmp);
    assert_eq!(xs, vec![30.0, 90.0]);
    assert!(vertical_hits(&d, 130.0).is_empty());
}
