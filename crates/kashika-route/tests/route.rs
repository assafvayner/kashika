use kashika_geom::Pt;
use kashika_route::*;

const WALL: &str = "diagram \"w\" size (460,240)\n\
box a \"A\" at (20,100) size (100,40)\n\
box wall \"W\" at (200,40) size (40,160)\n\
box b \"B\" at (320,100) size (100,40)\n";

#[test]
fn parses_reference_owner_and_side() {
    assert_eq!(
        parse_ref("pg.left@0.25+(0,4)"),
        (Some("pg".into()), Some(Side::Left))
    );
    assert_eq!(parse_ref("db.tl"), (Some("db".into()), None));
    assert_eq!(parse_ref("fan"), (Some("fan".into()), None));
    assert_eq!(parse_ref("(10,20)"), (None, None));
}

#[test]
fn resolves_points_through_the_model() {
    let (a, b) = resolve_endpoints(WALL, "a.top", "(300,20)").unwrap();
    assert_eq!(a.at, Pt::new(70.0, 100.0));
    assert_eq!(a.expr.as_deref(), Some("a.top"));
    assert_eq!(a.side, Some(Side::Top));
    assert_eq!(b.at, Pt::new(300.0, 20.0));
    assert_eq!(b.expr, None);
}

#[test]
fn bad_points_report_diagnostics() {
    match resolve_endpoints(WALL, "a.tpo", "b.top") {
        Err(Error::Diagnostics(d)) => assert_eq!(d[0].code, "unknown-anchor"),
        other => panic!("{other:?}"),
    }
}

fn scene(src: &str) -> kashika_model::Scene {
    let (s, d) = kashika_model::compile(src);
    assert!(d.is_empty(), "{d:#?}");
    s
}

const DIAG: &str = "diagram \"c\" size (420,220)\n\
box a at (20,20) size (100,40)\n\
box b at (300,160) size (100,40)\n";

#[test]
fn orthogonal_plans_include_lanes_around_boxes() {
    let s = scene(WALL);
    let (a, b) = resolve_endpoints(WALL, "a.top", "b.top").unwrap();
    let plans = generate(&s, &a, &b, Style::Corner);
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
    assert!(
        plans.iter().any(|p| p.hops == vec![Hop::To]),
        "aligned endpoints get a straight plan"
    );
}

#[test]
fn literals_are_written_in_the_line_frame() {
    let from = Endpoint::new("(120,140)", Pt::new(120.0, 140.0));
    let to = Endpoint::new("b.left", Pt::new(280.0, 160.0));
    let plan = Plan {
        hops: vec![
            Hop::X(Lane {
                value: 200.0,
                expr: None,
            }),
            Hop::Vh,
        ],
        lane: Some(("x", 200.0)),
    };
    assert_eq!(
        path_text(&from, &to, &plan, Pt::new(100.0, 100.0)),
        "(20,40) -> x 100 -> vh b.left"
    );
}

#[test]
fn bezier_plans_curve_both_ways_and_via_the_sides() {
    let s = scene(DIAG);
    let (a, b) = resolve_endpoints(DIAG, "a.right", "b.left").unwrap();
    let texts: Vec<String> = generate(&s, &a, &b, Style::Bezier)
        .iter()
        .map(|p| path_text(&a, &b, p, Pt::default()))
        .collect();
    assert!(
        texts.contains(&"a.right -> curve left 34 b.left".to_string()),
        "{texts:?}"
    );
    assert!(texts.contains(&"a.right -> curve right 68 b.left".to_string()));
    assert!(
        texts.contains(&"a.right -> curve via a.right+(76,0) b.left+(-76,0) b.left".to_string())
    );
}

#[test]
fn statements_and_numbers() {
    assert_eq!(
        statement("ab", "a.right -> b.left", "end=arrow"),
        "line ab a.right -> b.left end=arrow"
    );
    assert_eq!(
        statement("ab", "a.right -> b.left", ""),
        "line ab a.right -> b.left"
    );
    assert_eq!(
        (num(12.0), num(-0.0), num(0.256)),
        ("12".into(), "0".into(), "0.26".into())
    );
}

static OPTS: kashika_model::Options = kashika_model::Options {
    path: None,
    allow_path: false,
    icon_path: Vec::new(),
};

fn req<'a>(src: &'a str, from: &'a str, to: &'a str, style: Style) -> Request<'a> {
    Request {
        src,
        from: Some(from),
        to: Some(to),
        style,
        line: None,
        radius: 8.0,
        max: 3,
        opts: &OPTS,
    }
}

#[test]
fn routes_over_the_wall() {
    let s = suggest(&req(WALL, "a.top", "b.top", Style::Corner)).unwrap();
    assert_eq!(
        s.candidates.iter().filter(|c| c.segments <= 3).count(),
        1,
        "every other plan with at most 2 elbows enters a box or leaves the wrong way"
    );
    let c = &s.candidates[0];
    assert_eq!(
        c.statement,
        "line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow"
    );
    assert_eq!(
        (c.crossings, c.near, c.segments, c.length),
        (0, 0, 3, 444.0)
    );
    assert_eq!(
        c.corridor,
        Some(Corridor {
            axis: "y",
            band: [0.0, 34.0]
        })
    );
}

#[test]
fn detours_around_a_wall_between_facing_sides() {
    let mut r = req(WALL, "a.right", "b.left", Style::Corner);
    r.max = 20;
    let s = suggest(&r).unwrap();
    let c = &s.candidates[0];
    assert_eq!((c.crossings, c.near, c.segments), (0, 0, 5), "{c:?}");
    assert!(
        c.statement.starts_with("line a_b a.right -> x ") && c.statement.contains(" -> vh b.left"),
        "{}",
        c.statement
    );
    assert!(c.corridor.is_some(), "{c:?}");
    agrees_with_check(WALL, &s);
}

const QUEUE: &str = "diagram \"q\" size (520,340)\n\
box orders at (30,100) size (130,60)\n\
box queue at (230,100) size (170,60)\n\
box db at (370,200) size (100,100)\n";

#[test]
fn detours_around_a_box_into_a_top_anchor() {
    let mut r = req(QUEUE, "orders.right", "db.top", Style::Corner);
    r.max = 20;
    let s = suggest(&r).unwrap();
    let c = &s.candidates[0];
    assert_eq!((c.crossings, c.near, c.segments), (0, 0, 4), "{c:?}");
    assert_eq!(
        c.statement,
        "line orders_db orders.right -> x orders.tr+(12,0) -> y db.tl+(0,-12) -> hv db.top end=arrow"
    );
    agrees_with_check(QUEUE, &s);
}

#[test]
fn detours_in_arc_style_check_clean() {
    for (src, from, to) in [
        (WALL, "a.right", "b.left"),
        (QUEUE, "orders.right", "db.top"),
        (QUEUE, "db.left", "orders.bottom"),
    ] {
        for style in [Style::Corner, Style::Arc] {
            let mut r = req(src, from, to, style);
            r.max = 50;
            let s = suggest(&r).unwrap();
            assert_eq!(s.candidates[0].crossings, 0, "{from} {to}");
            agrees_with_check(src, &s);
        }
    }
}

#[test]
fn no_candidate_when_the_start_faces_a_box() {
    let src = "diagram \"b\" size (300,200)\n\
box a at (20,80) size (100,40)\n\
box c at (122,20) size (40,160)\n\
box b at (220,80) size (60,40)\n";
    let s = suggest(&req(src, "a.right", "b.left", Style::Corner)).unwrap();
    assert!(s.candidates.is_empty(), "{:?}", s.candidates);
}

#[test]
fn simple_paths_outrank_clean_detours() {
    let mut r = req(WALL, "a.top", "b.top", Style::Corner);
    r.max = 50;
    let s = suggest(&r).unwrap();
    assert_eq!(s.candidates[0].segments, 3);
    for c in s.candidates.iter().filter(|c| c.segments > 3) {
        assert!(
            s.candidates.iter().all(|d| d.segments > 3
                || d.crossings > c.crossings
                || d.near > c.near
                || d.length > c.length),
            "a simpler path is as good as {c:?}"
        );
    }
    let mut r = req(QUEUE, "orders.bottom", "db.left", Style::Corner);
    r.max = 50;
    let s = suggest(&r).unwrap();
    assert_eq!(
        s.candidates[0].statement,
        "line orders_db orders.bottom -> vh db.left end=arrow"
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
fn detour_search_stays_bounded_on_a_large_diagram() {
    let src = grid(10);
    let s = scene(&src);
    let (a, b) = resolve_endpoints(&src, "b0_0.right", "b9_9.left").unwrap();
    let start = std::time::Instant::now();
    let plans = generate(&s, &a, &b, Style::Corner);
    let took = start.elapsed();
    let detours = plans.iter().filter(|p| p.hops.len() >= 3).count();
    assert!(detours > 0 && detours <= MAX_DETOURS, "{detours} detours");
    assert!(plans.len() <= 3 + 2 * (2 * 100 + 1) + MAX_DETOURS);
    assert!(took.as_secs() < 2, "generating plans took {took:?}");

    let src = grid(5);
    let mut r = req(&src, "b0_0.right", "b4_4.left", Style::Corner);
    r.max = 5;
    let s = suggest(&r).unwrap();
    assert_eq!(
        (s.candidates[0].crossings, s.candidates[0].segments),
        (0, 5)
    );
    agrees_with_check(&src, &s);
}

#[test]
fn counts_crossings_with_other_lines() {
    let src = format!("{WALL}line blocker (160,10) -> (160,230)\n");
    let s = suggest(&req(&src, "a.top", "b.top", Style::Corner)).unwrap();
    let c = &s.candidates[0];
    assert_eq!(c.crossings, 1);
    assert_eq!(c.crossed[0].line, "blocker");
    assert_eq!(c.crossed[0].at, Pt::new(160.0, 28.0));
}

#[test]
fn counts_near_misses_along_a_box_edge() {
    let src = "diagram \"n\" size (300,200)\n\
box a at (20,20) size (40,40)\n\
box b at (220,20) size (40,40)\n\
box c at (100,76) size (40,40)\n";
    let mut r = req(src, "a.bottom", "b.bottom", Style::Corner);
    r.max = 50;
    let s = suggest(&r).unwrap();
    let close = s
        .candidates
        .iter()
        .find(|c| c.statement.contains("a.bottom -> y a.bl+(0,12) -> hv"))
        .expect("a lane just below a and b");
    assert!(close.near >= 1, "lane at y=72 runs 4px above c: {close:?}");
}

#[test]
fn avoids_a_lane_on_a_group_border() {
    let src = "diagram \"g\" size (440,240)\n\
box a at (20,20) size (100,40)\n\
group vpc \"VPC\" at (200,110) size (220,120) style=dashed {\n}\n\
box b at (300,160) size (100,40)\n";
    let s = suggest(&req(src, "a.bottom", "b.top", Style::Corner)).unwrap();
    let border = s
        .candidates
        .iter()
        .find(|c| c.statement.contains("-> y 110 ->"))
        .expect("the midpoint lane on the VPC's top border");
    assert_eq!(border.near, 1, "{border:?}");
    assert_eq!(
        s.candidates[0].statement,
        "line a_b a.bottom -> y a.bl+(0,12) -> hv b.top end=arrow"
    );
    assert_eq!(
        s.candidates[0].corridor,
        Some(Corridor {
            axis: "y",
            band: [66.0, 104.0]
        })
    );
}

#[test]
fn default_id_avoids_existing_ids() {
    let src = format!("{WALL}junction a_b at (10,10)\n");
    let s = suggest(&req(&src, "a.top", "b.top", Style::Corner)).unwrap();
    assert!(s.candidates[0].statement.starts_with("line a_b_2 "));
}

#[test]
fn arc_adds_radius_and_counts_corners() {
    let s = suggest(&req(WALL, "a.top", "b.top", Style::Arc)).unwrap();
    let c = &s.candidates[0];
    assert!(
        c.statement.ends_with("end=arrow radius=8"),
        "{}",
        c.statement
    );
    assert_eq!(c.segments, 3);
}

#[test]
fn bezier_suggestions_are_curves_that_leave_and_enter_square() {
    let s = suggest(&req(DIAG, "a.right", "b.left", Style::Bezier)).unwrap();
    assert!(!s.candidates.is_empty());
    assert!(s
        .candidates
        .iter()
        .all(|c| c.statement.contains(" -> curve ")));
    assert!(s.candidates[0]
        .statement
        .contains("curve via a.right+(76,0) b.left+(-76,0)"));
}

#[test]
fn collinear_lanes_merge_into_the_straight_path() {
    let src = "diagram \"s\" size (300,120)\n\
box a at (20,20) size (40,40)\n\
box b at (220,20) size (40,40)\n";
    let mut r = req(src, "a.right", "b.left", Style::Corner);
    r.max = 20;
    let s = suggest(&r).unwrap();
    let statements: Vec<&str> = s
        .candidates
        .iter()
        .filter(|c| c.segments <= 3)
        .map(|c| c.statement.as_str())
        .collect();
    assert_eq!(statements, ["line a_b a.right -> b.left end=arrow"]);
}

#[test]
fn curves_leave_and_enter_side_anchors_along_the_normal() {
    let mut r = req(DIAG, "a.right", "b.top", Style::Bezier);
    r.max = 20;
    let s = suggest(&r).unwrap();
    assert!(!s.candidates.is_empty());
    for c in &s.candidates {
        let line_scene = scene(&format!("{DIAG}{}\n", c.statement));
        let path = &line_scene.lines[0].path;
        let (start, end) = (path[0].start_dir(), path[path.len() - 1].end_dir());
        assert!(
            start.x > 0.999,
            "leaves a.right horizontally: {}",
            c.statement
        );
        assert!(end.y > 0.999, "enters b.top vertically: {}", c.statement);
    }
}

const GROUPED: &str = "diagram \"g\" size (400,300)\n\
group g \"G\" at (100,100) size (280,180) {\n\
  box a \"A\" at (20,40) size (60,40)\n\
  box b \"B\" at (180,40) size (60,40)\n\
  line ab a.right -> (180,60) end=open label=\"x\" radius=4\n\
}\n";

fn reroute(src: &str, id: &str, style: Style) -> Result<Suggestion, Error> {
    suggest(&Request {
        src,
        from: None,
        to: None,
        style,
        line: Some(id),
        radius: 8.0,
        max: 3,
        opts: &OPTS,
    })
}

#[test]
fn reroutes_an_existing_line_in_its_frame_and_keeps_attributes() {
    let s = reroute(GROUPED, "ab", Style::Corner).unwrap();
    assert_eq!(
        s.candidates[0].statement,
        "line ab a.right -> (180,60) end=open label=\"x\""
    );
    assert_eq!(s.from.at, Pt::new(180.0, 160.0));
    assert_eq!(s.to.at, Pt::new(280.0, 160.0));
}

#[test]
fn rerouted_line_is_not_its_own_obstacle() {
    let src = format!("{WALL}line ab a.top -> y 28 -> hv b.top\n");
    let s = reroute(&src, "ab", Style::Arc).unwrap();
    assert_eq!(s.candidates[0].crossings, 0);
    assert_eq!(s.candidates[0].near, 0);
    assert!(s.candidates[0].statement.starts_with("line ab a.top -> "));
}

#[test]
fn unknown_line_is_a_usage_error() {
    assert!(matches!(
        reroute(WALL, "nope", Style::Corner),
        Err(Error::Usage(_))
    ));
}

#[test]
fn mixed_points_in_a_group_keep_their_text_and_frame() {
    let src = "diagram \"m\" size (400,300)\n\
group g \"G\" at (100,100) size (280,180) {\n\
  box a \"A\" at (20,40) size (60,40)\n\
  box b \"B\" at (180,100) size (60,40)\n\
  line ab a.bottom -> (b.left.x, 120) end=arrow\n\
}\n";
    let s = reroute(src, "ab", Style::Corner).unwrap();
    assert_eq!(s.to.expr.as_deref(), Some("(b.left.x, 120)"));
    assert_eq!(s.to.at, Pt::new(280.0, 220.0));
    assert!(!s.candidates.is_empty());
    for c in &s.candidates {
        assert!(
            c.statement.ends_with("(b.left.x, 120) end=arrow"),
            "{}",
            c.statement
        );
        let replaced = src.replace(
            "  line ab a.bottom -> (b.left.x, 120) end=arrow",
            &c.statement,
        );
        let (scene, diags) = kashika_model::compile(&replaced);
        assert!(diags.is_empty(), "{}: {diags:#?}", c.statement);
        let path = &scene.lines[0].path;
        assert_eq!(path[path.len() - 1].to(), Pt::new(280.0, 220.0));
    }
}

/// Every candidate's numbers are what `kashika check` reports once the statement is pasted.
fn agrees_with_check(src: &str, s: &Suggestion) {
    assert!(!s.candidates.is_empty(), "no candidates for\n{src}");
    for c in &s.candidates {
        let id = c.statement["line ".len()..].split(' ').next().unwrap();
        let (scene, diags) = kashika_model::compile(&format!("{src}{}\n", c.statement));
        assert!(diags.is_empty(), "{}: {diags:#?}", c.statement);
        let mine: Vec<_> = kashika_check::check(&scene)
            .into_iter()
            .filter(|d| d.elements.iter().any(|e| e.id == id))
            .collect();
        let count = |codes: &[&str]| mine.iter().filter(|d| codes.contains(&d.code)).count();
        assert_eq!(count(&["through"]), 0, "{}: {mine:#?}", c.statement);
        assert_eq!(count(&["cross"]), c.crossings, "{}: {mine:#?}", c.statement);
        assert_eq!(
            count(&["near", "shared-segment"]),
            c.near,
            "{}: {mine:#?}",
            c.statement
        );
    }
}

const DIAMOND: &str = "diagram \"d\" size (320,240)\n\
box a at (20,70) size (60,40)\n\
box b at (150,0) size (80,40)\n\
box d at (160,80) size (120,120) shape=diamond\n";

#[test]
fn routes_through_the_bounding_box_corner_of_a_diamond() {
    let s = suggest(&req(DIAMOND, "a.right", "b.bottom", Style::Corner)).unwrap();
    let statements: Vec<&str> = s
        .candidates
        .iter()
        .filter(|c| c.segments <= 3)
        .map(|c| c.statement.as_str())
        .collect();
    assert_eq!(statements, ["line a_b a.right -> hv b.bottom end=arrow"]);
    assert_eq!((s.candidates[0].crossings, s.candidates[0].near), (0, 0));
    agrees_with_check(DIAMOND, &s);
}

#[test]
fn routes_over_a_cylinder_and_an_image() {
    for wall in [
        "box wall \"W\" at (200,40) size (40,160) shape=cylinder",
        "image wall \"server\" at (200,40) size (40,160)",
    ] {
        let src = WALL.replace("box wall \"W\" at (200,40) size (40,160)", wall);
        let s = suggest(&req(&src, "a.top", "b.top", Style::Corner)).unwrap();
        assert_eq!(
            s.candidates[0].statement, "line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow",
            "{wall}"
        );
        agrees_with_check(&src, &s);
    }
}

#[test]
fn candidate_numbers_match_kashika_check() {
    let blocked = format!("{WALL}line blocker (160,10) -> (160,230)\n");
    let near = "diagram \"n\" size (300,200)\n\
box a at (20,20) size (40,40)\n\
box b at (220,20) size (40,40)\n\
box c at (100,76) size (40,40)\n";
    let cases: [(&str, &str, &str, Style); 6] = [
        (WALL, "a.top", "b.top", Style::Corner),
        (WALL, "a.top", "b.top", Style::Arc),
        (&blocked, "a.top", "b.top", Style::Corner),
        (near, "a.bottom", "b.bottom", Style::Corner),
        (DIAG, "a.right", "b.left", Style::Bezier),
        (DIAG, "a.bottom", "b.left", Style::Corner),
    ];
    for (src, from, to, style) in cases {
        let mut r = req(src, from, to, style);
        r.max = 20;
        agrees_with_check(src, &suggest(&r).unwrap());
    }
}

#[test]
fn reroutes_a_line_through_a_wall_with_a_detour_in_its_group() {
    let line = "  line ab a.right -> b.left end=arrow";
    let src = format!(
        "diagram \"g\" size (560,400)\n\
group g \"G\" at (60,60) size (480,300) {{\n\
  box a at (20,120) size (100,40)\n\
  box wall at (200,40) size (40,220)\n\
  box b at (320,120) size (100,40)\n\
{line}\n\
}}\n"
    );
    let r = Request {
        src: &src,
        from: None,
        to: None,
        style: Style::Corner,
        line: Some("ab"),
        radius: 8.0,
        max: 10,
        opts: &OPTS,
    };
    let s = suggest(&r).unwrap();
    assert_eq!(
        (s.candidates[0].crossings, s.candidates[0].segments),
        (0, 5)
    );
    for c in &s.candidates {
        assert!(
            c.statement.starts_with("line ab a.right -> "),
            "{}",
            c.statement
        );
        let replaced = src.replace(line, &format!("  {}", c.statement));
        let (scene, diags) = kashika_model::compile(&replaced);
        assert!(diags.is_empty(), "{}: {diags:#?}", c.statement);
        let through = kashika_check::check(&scene)
            .into_iter()
            .filter(|d| d.code == "through")
            .count();
        assert_eq!(through, 0, "{}", c.statement);
    }
}
