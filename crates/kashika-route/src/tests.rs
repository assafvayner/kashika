use std::time::Instant;

use kashika_check::ctx::Ctx;
use kashika_geom::Pt;
use kashika_model::{Options, Scene};

use crate::candidates::{
    Hop, Lane, MAX_DETOURS, MAX_LANE_PLANS, Plan, corners, doubles_back, generate, key,
};
use crate::endpoint::resolve_endpoints;
use crate::text::{path_text, statement};
use crate::{Endpoint, Error, Side, Style};
use kashika_geom::fmt_num as num;

const WALL: &str = "diagram \"w\" size (460,240)\n\
box a \"A\" at (20,100) size (100,40)\n\
box wall \"W\" at (200,40) size (40,160)\n\
box b \"B\" at (320,100) size (100,40)\n";

const DIAG: &str = "diagram \"c\" size (420,220)\n\
box a at (20,20) size (100,40)\n\
box b at (300,160) size (100,40)\n";

fn scene(src: &str) -> Scene {
    let (s, d) = kashika_model::compile(src);
    assert!(d.is_empty(), "{d:#?}");
    s
}

fn ends(src: &str, from: &str, to: &str) -> (Endpoint, Endpoint) {
    resolve_endpoints(src, from, to, &Options::default()).unwrap()
}

fn plan_for(src: &str, from: &str, to: &str, style: Style) -> (Endpoint, Endpoint, Vec<Plan>) {
    let s = scene(src);
    let (a, b) = ends(src, from, to);
    let plans = generate(&Ctx::new(&s), &a, &b, style, &[]);
    (a, b, plans)
}

#[test]
fn resolves_points_through_the_model() {
    let (a, b) = ends(WALL, "a.top", "(300,20)");
    assert_eq!(a.at, Pt::new(70.0, 100.0));
    assert_eq!(a.expr.as_deref(), Some("a.top"));
    assert_eq!((a.owner.as_deref(), a.side), (Some("a"), Some(Side::Top)));
    assert_eq!(b.at, Pt::new(300.0, 20.0));
    assert_eq!((b.expr, b.owner, b.side), (None, None, None));
}

#[test]
fn endpoints_parse_like_points_in_a_line() {
    let src = format!("{WALL}junction fan at (160,20)\n");
    let (a, b) = ends(&src, "a.right + (0, 4)", "b.left@0.25-(0,4)");
    assert_eq!(a.expr.as_deref(), Some("a.right + (0, 4)"));
    assert_eq!((a.owner.as_deref(), a.side), (Some("a"), Some(Side::Right)));
    assert_eq!(a.anchor, Some(("a.right".into(), Pt::new(0.0, 4.0))));
    assert_eq!(a.at, Pt::new(120.0, 124.0));
    assert_eq!((b.owner.as_deref(), b.side), (Some("b"), Some(Side::Left)));
    assert_eq!(b.anchor, Some(("b.left@0.25".into(), Pt::new(0.0, -4.0))));
    assert_eq!(b.at, Pt::new(320.0, 106.0));

    let (a, b) = ends(&src, "fan", "b.top-left");
    assert_eq!((a.owner.as_deref(), a.side), (Some("fan"), None));
    assert_eq!((b.owner.as_deref(), b.side), (Some("b"), None));

    let (a, _) = ends(&src, "(b.left.x, 30)", "b.top");
    assert_eq!(a.expr.as_deref(), Some("(b.left.x, 30)"));
    assert_eq!((a.owner, a.side), (None, None));
    assert_eq!(a.at, Pt::new(320.0, 30.0));

    let (a, b) = ends(
        &src,
        "(a.right+(0,4))+(0,2)",
        "((b.left.x, b.top.y)+(0,20))",
    );
    assert_eq!(a.expr.as_deref(), Some("(a.right+(0,4))+(0,2)"));
    assert_eq!(a.anchor, Some(("a.right".into(), Pt::new(0.0, 6.0))));
    assert_eq!(a.at, Pt::new(120.0, 126.0));
    assert_eq!(b.expr.as_deref(), Some("((b.left.x, b.top.y)+(0,20))"));
    assert_eq!(b.at, Pt::new(320.0, 120.0));

    let (a, _) = ends(&src, "(10,20)+(5,0)", "b.top");
    assert_eq!((a.expr, a.owner), (None, None));
    assert_eq!(a.at, Pt::new(15.0, 20.0));
}

#[test]
fn bad_points_report_diagnostics_or_usage() {
    let opts = Options::default();
    match resolve_endpoints(WALL, "a.tpo", "b.top", &opts) {
        Err(Error::Diagnostics(d)) => assert_eq!(d[0].code, "unknown-anchor"),
        other => panic!("{other:?}"),
    }
    for (from, to) in [
        ("a.top -> (10,10)", "b.top"),
        ("a.top", "b.top\nbox z at (0,0) size (4,4)"),
    ] {
        assert!(
            matches!(
                resolve_endpoints(WALL, from, to, &opts),
                Err(Error::Usage(_))
            ),
            "{from:?} -> {to:?}"
        );
    }
}

#[test]
fn orthogonal_plans_include_lanes_around_boxes() {
    let (a, b, plans) = plan_for(WALL, "a.top", "b.top", Style::Corner);
    let lane = Hop::Y(Lane {
        value: 28.0,
        expr: Some("wall.tl+(0,-12)".into()),
    });
    let over = plans
        .iter()
        .find(|p| p.hops.first() == Some(&lane))
        .expect("a lane above the wall");
    assert_eq!(
        path_text(&a, &b, over, Pt::default()),
        "a.top -> y wall.tl+(0,-12) -> hv b.top"
    );
    let (_, _, plans) = plans_aligned();
    assert!(
        plans.iter().any(|p| p.hops == vec![Hop::To]),
        "aligned endpoints get a straight plan"
    );
}

fn plans_aligned() -> (Endpoint, Endpoint, Vec<Plan>) {
    plan_for(WALL, "a.right", "b.left", Style::Corner)
}

#[test]
fn plans_that_double_back_are_dropped() {
    assert!(doubles_back(&[
        Pt::new(0.0, 0.0),
        Pt::new(0.0, 0.0),
        Pt::new(10.0, 0.0)
    ]));
    assert!(doubles_back(&[
        Pt::new(0.0, 0.0),
        Pt::new(10.0, 0.0),
        Pt::new(4.0, 0.0)
    ]));
    assert!(!doubles_back(&[
        Pt::new(0.0, 0.0),
        Pt::new(10.0, 0.0),
        Pt::new(10.0, 4.0)
    ]));

    let (a, b, plans) = plans_aligned();
    assert!(!plans.is_empty());
    for p in &plans {
        let pts = corners(a.at, b.at, p).unwrap();
        assert!(
            !doubles_back(&pts),
            "{}",
            path_text(&a, &b, p, Pt::default())
        );
    }
    let lane_at_ends = Hop::Y(Lane {
        value: 120.0,
        expr: None,
    });
    assert!(!plans.iter().any(|p| p.hops.first() == Some(&lane_at_ends)));
}

#[test]
fn plans_draw_distinct_paths() {
    for (src, from, to) in [
        (WALL, "a.top", "b.top"),
        (WALL, "a.right", "b.left"),
        (DIAG, "a.bottom", "b.left"),
    ] {
        let (a, b, plans) = plan_for(src, from, to, Style::Corner);
        let mut keys: Vec<_> = plans
            .iter()
            .map(|p| key(corners(a.at, b.at, p).unwrap()))
            .collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n, "{from} -> {to}");
    }
}

#[test]
fn lanes_at_one_position_keep_the_endpoint_box() {
    let src = "diagram \"l\" size (420,300)\n\
box c at (300,20) size (60,40)\n\
box a at (20,20) size (60,40)\n\
box b at (300,200) size (60,40)\n";
    let (a, b, plans) = plan_for(src, "a.right", "b.left", Style::Corner);
    let lanes: Vec<String> = plans
        .iter()
        .filter_map(|p| match p.hops.first() {
            Some(Hop::X(l)) if p.hops.len() == 2 && l.value == 288.0 => {
                Some(path_text(&a, &b, p, Pt::default()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(lanes, ["a.right -> x b.tl+(-12,0) -> vh b.left"]);
}

#[test]
fn bezier_plans_skip_curves_that_cannot_leave_square() {
    let (a, b, plans) = plan_for(DIAG, "a.right", "b.left", Style::Bezier);
    let texts: Vec<String> = plans
        .iter()
        .map(|p| path_text(&a, &b, p, Pt::default()))
        .collect();
    assert_eq!(
        texts,
        ["a.right -> curve via a.right+(76,0) b.left+(-76,0) b.left"]
    );

    let (a, b, plans) = plan_for(DIAG, "a.tr", "b.bl", Style::Bezier);
    let texts: Vec<String> = plans
        .iter()
        .map(|p| path_text(&a, &b, p, Pt::default()))
        .collect();
    assert_eq!(
        texts,
        [
            "a.tr -> curve left 38 b.bl",
            "a.tr -> curve right 38 b.bl",
            "a.tr -> curve left 76 b.bl",
            "a.tr -> curve right 76 b.bl",
        ]
    );

    let (a, b, plans) = plan_for(DIAG, "a.right+(0,4)", "b.left", Style::Bezier);
    assert_eq!(
        path_text(&a, &b, &plans[0], Pt::default()),
        "a.right+(0,4) -> curve via a.right+(75,4) b.left+(-75,0) b.left"
    );
}

#[test]
fn literals_are_written_in_the_line_frame() {
    let from = Endpoint::literal(Pt::new(120.0, 140.0));
    let to = Endpoint {
        expr: Some("b.left".into()),
        owner: Some("b".into()),
        side: Some(Side::Left),
        ..Endpoint::literal(Pt::new(280.0, 160.0))
    };
    let plan = |hops: Vec<Hop>| Plan { hops, lane: None };
    let lane = |value| Lane { value, expr: None };
    let origin = Pt::new(100.0, 100.0);
    assert_eq!(
        path_text(
            &from,
            &to,
            &plan(vec![Hop::X(lane(220.0)), Hop::Vh]),
            origin
        ),
        "(20,40) -> x 120 -> vh b.left"
    );
    assert_eq!(
        path_text(
            &from,
            &to,
            &plan(vec![Hop::X(lane(200.0)), Hop::Vh]),
            origin
        ),
        "(20,40) -> hvh b.left"
    );
    assert_eq!(
        path_text(
            &from,
            &to,
            &plan(vec![Hop::Y(lane(150.0)), Hop::Hv]),
            origin
        ),
        "(20,40) -> vhv b.left"
    );
    let detour = vec![
        Hop::X(lane(60.0)),
        Hop::Y(lane(300.0)),
        Hop::X(lane(170.0)),
        Hop::Vh,
    ];
    assert_eq!(
        path_text(&from, &to, &plan(detour), origin),
        "(20,40) -> x -40 -> y 200 -> hvh b.left"
    );
}

#[test]
fn statements_and_numbers() {
    assert_eq!(
        statement(false, "ab", "a.right -> b.left", "end=arrow"),
        "line ab a.right -> b.left end=arrow"
    );
    assert_eq!(
        statement(false, "ab", "a.right -> b.left", ""),
        "line ab a.right -> b.left"
    );
    assert_eq!(
        statement(true, "ab", "a.right -> b.left", "label=\"x\""),
        "arrow ab a.right -> b.left label=\"x\""
    );
    assert_eq!(
        (num(12.0), num(-0.0), num(0.256)),
        ("12".into(), "0".into(), "0.26".into())
    );
}

/// An `n` × `n` grid of boxes with a line between the first two of each row.
fn grid(n: usize) -> String {
    let side = 40 + 120 * n;
    let mut src = format!("diagram \"big\" size ({side},{side})\n");
    for i in 0..n {
        for j in 0..n {
            src.push_str(&format!(
                "box b{i}_{j} at ({},{}) size (80,60)\n",
                40 + 120 * j,
                40 + 120 * i
            ));
        }
        src.push_str(&format!("line r{i} b{i}_0.right -> b{i}_1.left\n"));
    }
    src
}

#[test]
fn plan_search_stays_bounded_on_a_large_diagram() {
    let src = grid(20);
    let s = scene(&src);
    let ctx = Ctx::new(&s);
    for (from, to) in [("b0_0.right", "b19_19.left"), ("b0_0.bottom", "b1_0.top")] {
        let (a, b) = ends(&src, from, to);
        let start = Instant::now();
        let plans = generate(&ctx, &a, &b, Style::Corner, &[]);
        let took = start.elapsed();
        let lanes = plans.iter().filter(|p| p.hops.len() == 2).count();
        let detours = plans.iter().filter(|p| p.hops.len() >= 3).count();
        assert!(lanes <= MAX_LANE_PLANS, "{lanes} one-lane plans");
        assert!(detours <= MAX_DETOURS, "{detours} detours");
        assert!(took.as_secs() < 2, "generating plans took {took:?}");
    }
}
