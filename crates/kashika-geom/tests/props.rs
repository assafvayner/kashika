use kashika_geom::{
    PathSeg, Pt, convex_hull, cross, flatten, length, polygons_overlap, seg_hit, seg_seg_dist,
};
use proptest::prelude::*;

fn pt(range: f64) -> impl Strategy<Value = Pt> {
    (-range..range, -range..range).prop_map(|(x, y)| Pt::new(x, y))
}

/// Small integer coordinates, so collinear, touching and shared-endpoint cases are common.
fn grid_pt() -> impl Strategy<Value = Pt> {
    (0..6i32, 0..6i32).prop_map(|(x, y)| Pt::new(x as f64, y as f64))
}

fn any_pt() -> impl Strategy<Value = Pt> {
    prop_oneof![pt(100.0), grid_pt()]
}

fn box_pt() -> impl Strategy<Value = Pt> {
    (0.0..1000.0, 0.0..1000.0).prop_map(|(x, y)| Pt::new(x, y))
}

fn curve() -> impl Strategy<Value = PathSeg> {
    prop_oneof![
        (box_pt(), box_pt(), box_pt()).prop_map(|(from, ctrl, to)| PathSeg::Quad {
            from,
            ctrl,
            to
        }),
        (box_pt(), box_pt(), box_pt(), box_pt()).prop_map(|(from, c1, c2, to)| PathSeg::Cubic {
            from,
            c1,
            c2,
            to
        }),
    ]
}

fn convex_polygon() -> impl Strategy<Value = Vec<Pt>> {
    prop::collection::vec(any_pt(), 3..8)
        .prop_map(|pts| convex_hull(&pts))
        .prop_filter("needs three hull points", |h| h.len() >= 3)
}

fn chord_length(seg: &PathSeg, params: impl Iterator<Item = f64>) -> f64 {
    let mut prev = seg.from();
    let mut acc = 0.0;
    for t in params {
        let p = seg.eval(t);
        acc += prev.dist(p);
        prev = p;
    }
    acc
}

const STEPS: usize = 10_000;

fn numeric_length(seg: &PathSeg) -> f64 {
    chord_length(seg, (1..=STEPS).map(|k| k as f64 / STEPS as f64))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn seg_hit_is_symmetric(
        a in any_pt(), b in any_pt(), c in any_pt(), d in any_pt(),
        eps in prop_oneof![Just(0.0), Just(0.5)],
    ) {
        prop_assert_eq!(
            seg_hit(a, b, c, d, eps).is_some(),
            seg_hit(c, d, a, b, eps).is_some()
        );
    }

    #[test]
    fn polygons_overlap_is_symmetric(a in convex_polygon(), b in convex_polygon()) {
        prop_assert_eq!(polygons_overlap(&a, &b), polygons_overlap(&b, &a));
    }

    #[test]
    fn seg_seg_dist_is_symmetric(a in any_pt(), b in any_pt(), c in any_pt(), d in any_pt()) {
        let (l, r) = (seg_seg_dist(a, b, c, d), seg_seg_dist(c, d, a, b));
        prop_assert!((l - r).abs() <= 1e-9, "{} vs {}", l, r);
    }

    #[test]
    fn flattened_length_is_within_one_percent(seg in curve()) {
        let exact = numeric_length(&seg);
        let flat = length(&[seg]);
        prop_assert!(
            (flat - exact).abs() <= 0.01 * exact,
            "flattened {} vs numeric {} ({} points)", flat, exact, flatten(&[seg]).len()
        );
    }

    #[test]
    fn split_halves_rejoin(seg in curve(), t in 0.0..1.0f64) {
        let (a, b) = seg.split(t);
        prop_assert_eq!(a.from(), seg.from());
        prop_assert_eq!(a.to(), b.from());
        prop_assert_eq!(b.to(), seg.to());
        prop_assert!(a.to().dist(seg.eval(t)) <= 1e-9);
        let halves = numeric_length(&a) + numeric_length(&b);
        let whole = chord_length(
            &seg,
            (1..=STEPS)
                .map(|k| t * k as f64 / STEPS as f64)
                .chain((1..=STEPS).map(|k| t + (1.0 - t) * k as f64 / STEPS as f64)),
        );
        prop_assert!((halves - whole).abs() <= 1e-6, "{} vs {}", halves, whole);
    }

    #[test]
    fn hull_is_convex_and_holds_every_point(pts in prop::collection::vec(any_pt(), 3..12)) {
        let h = convex_hull(&pts);
        prop_assume!(h.len() >= 3);
        let n = h.len();
        for i in 0..n {
            let (a, b, c) = (h[i], h[(i + 1) % n], h[(i + 2) % n]);
            prop_assert!(cross(b - a, c - b) > 0.0, "turn at {:?} in {:?}", b, h);
        }
        for &p in &pts {
            for i in 0..n {
                let (a, b) = (h[i], h[(i + 1) % n]);
                prop_assert!(
                    cross(b - a, p - a) >= -1e-9 * (1.0 + (b - a).length() * (p - a).length()),
                    "{:?} outside edge {:?}-{:?} of {:?}", p, a, b, h
                );
            }
        }
    }
}
