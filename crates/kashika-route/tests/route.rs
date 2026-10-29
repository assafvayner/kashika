use kashika_geom::Pt;
use kashika_route::{Axis, Corridor, Error, Request, Style, Suggestion, suggest};

const WALL: &str = "diagram \"w\" size (460,240)\n\
box a \"A\" at (20,100) size (100,40)\n\
box wall \"W\" at (200,40) size (40,160)\n\
box b \"B\" at (320,100) size (100,40)\n";

fn scene(src: &str) -> kashika_model::Scene {
    let (s, d) = kashika_model::compile(src);
    assert!(d.is_empty(), "{d:#?}");
    s
}

const DIAG: &str = "diagram \"c\" size (420,220)\n\
box a at (20,20) size (100,40)\n\
box b at (300,160) size (100,40)\n";

static OPTS: kashika_model::Options = kashika_model::Options {
    path: None,
    allow_path: false,
    icon_path: Vec::new(),
    include: Vec::new(),
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
            axis: Axis::Y,
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
fn route_stays_fast_on_a_large_diagram() {
    let src = grid(20);
    let start = std::time::Instant::now();
    let s = suggest(&req(&src, "b0_0.bottom", "b1_0.top", Style::Corner)).unwrap();
    let took = start.elapsed();
    assert_eq!(
        s.candidates[0].statement,
        "line b0_0_b1_0 b0_0.bottom -> b1_0.top end=arrow"
    );
    assert!(took.as_secs() < 10, "route took {took:?}");

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
    let mut r = req(src, "a.bottom", "b.top", Style::Corner);
    r.max = 10;
    let s = suggest(&r).unwrap();
    let border = s
        .candidates
        .iter()
        .find(|c| c.statement.contains("-> vhv b.top"))
        .expect("the midpoint lane on the VPC's top border");
    assert_eq!(border.near, 1, "{border:?}");
    assert_eq!(
        s.candidates[0].statement,
        "line a_b a.bottom -> y a.bl+(0,12) -> hv b.top end=arrow"
    );
    assert_eq!(
        s.candidates[0].corridor,
        Some(Corridor {
            axis: Axis::Y,
            band: [66.0, 104.0]
        })
    );
}

const TRANSIT: &str = "diagram \"groups\" size (560,260)\n\
box a \"A\" at (20,100) size (80,50)\n\
box b \"B\" at (460,100) size (80,50)\n\
group ns \"namespace\" at (170,40) size (220,170) {\n\
  box p \"pod\" at (70,110) size (80,40)\n\
}\n\
line ab a.right -> b.left end=arrow\n";

#[test]
fn detours_around_a_group_instead_of_transiting_it() {
    let mut r = reroute_req(TRANSIT, "ab");
    r.max = 20;
    let s = suggest(&r).unwrap();
    let c = &s.candidates[0];
    assert_eq!((c.crossings, c.transits, c.near), (0, 0, 0), "{c:?}");
    assert!(c.transited.is_empty(), "{c:?}");
    let straight = s
        .candidates
        .iter()
        .find(|c| c.statement == "line ab a.right -> b.left end=arrow")
        .expect("the straight path, ranked lower");
    assert_eq!((straight.crossings, straight.transits), (0, 1));
    assert_eq!(straight.transited[0].group, "ns");
    assert_eq!(straight.transited[0].at, Pt::new(170.0, 125.0));
    let src = TRANSIT.replace("line ab a.right -> b.left end=arrow\n", "");
    agrees_with_check(&src, &s);
}

const CROSS_OR_TRANSIT: &str = "diagram \"t\" size (490,260)\n\
box a at (10,20) size (90,40)\n\
box b at (400,200) size (80,40)\n\
group g \"G\" at (150,100) size (200,60) {\n}\n\
line w1 (0,130) -> (140,130)\n\
line w2 (360,130) -> (490,130)\n";

#[test]
fn a_transit_ranks_the_same_as_a_crossing() {
    let mut r = req(CROSS_OR_TRANSIT, "a.right", "b.left", Style::Corner);
    r.max = 20;
    let s = suggest(&r).unwrap();
    let rows: Vec<(usize, usize, usize, usize, &str)> = s
        .candidates
        .iter()
        .map(|c| {
            (
                c.crossings,
                c.transits,
                c.near,
                c.segments,
                c.statement.as_str(),
            )
        })
        .collect();
    assert!(
        rows.iter()
            .all(|&(c, t, n, seg, _)| c + t == 1 && n == 0 && seg == 3),
        "every lane crosses `w1` or `w2`, or transits `g`, with the same length: {rows:#?}"
    );
    assert_eq!(
        rows[..3],
        [
            (
                1,
                0,
                0,
                3,
                "line a_b a.right -> x a.tr+(12,0) -> vh b.left end=arrow"
            ),
            (
                1,
                0,
                0,
                3,
                "line a_b a.right -> x b.tl+(-12,0) -> vh b.left end=arrow"
            ),
            (0, 1, 0, 3, "line a_b a.right -> hvh b.left end=arrow"),
        ],
        "a tie, broken by lanes on the endpoints' own boxes: {rows:#?}"
    );
    assert_eq!(s.candidates[2].transited[0].group, "g");
    agrees_with_check(CROSS_OR_TRANSIT, &s);
}

const STACKED: &str = "diagram \"s\" size (600,400)\n\
group upper \"U\" at (40,20) size (300,160) {\n}\n\
group lower \"L\" at (40,220) size (520,160) {\n\
  box t at (60,10) size (80,40)\n\
}\n\
box p at (420,40) size (100,50)\n";

#[test]
fn one_lane_through_the_gap_between_two_groups() {
    let mut r = req(STACKED, "p.bottom", "t.top", Style::Corner);
    r.max = 20;
    let s = suggest(&r).unwrap();
    let c = &s.candidates[0];
    assert_eq!(
        (c.crossings, c.transits, c.near, c.segments),
        (0, 0, 0, 3),
        "{c:?}"
    );
    assert_eq!(
        c.statement,
        "line p_t p.bottom -> y 200 -> hv t.top end=arrow"
    );
    assert_eq!(
        c.corridor,
        Some(Corridor {
            axis: Axis::Y,
            band: [186.0, 214.0]
        })
    );
    agrees_with_check(STACKED, &s);
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
    assert!(
        s.candidates
            .iter()
            .all(|c| c.statement.contains(" -> curve "))
    );
    assert!(
        s.candidates[0]
            .statement
            .contains("curve via a.right+(76,0) b.left+(-76,0)")
    );
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

fn reroute_req<'a>(src: &'a str, id: &'a str) -> Request<'a> {
    Request {
        src,
        from: None,
        to: None,
        style: Style::Corner,
        line: Some(id),
        radius: 8.0,
        max: 3,
        opts: &OPTS,
    }
}

fn reroute(src: &str, id: &str, style: Style) -> Result<Suggestion, Error> {
    suggest(&Request {
        style,
        ..reroute_req(src, id)
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
            count(&["group-transit"]),
            c.transits,
            "{}: {mine:#?}",
            c.statement
        );
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

#[test]
fn crossings_name_unnamed_lines_as_check_does() {
    let src = format!("{WALL}line ab a.top -> b.top\nline (160,10) -> (160,230)\n");
    let s = reroute(&src, "ab", Style::Corner).unwrap();
    let c = &s.candidates[0];
    assert_eq!(c.crossings, 1, "{c:?}");
    assert_eq!(c.crossed[0].line, "line@L6");
    let replaced = src.replace("line ab a.top -> b.top", &c.statement);
    let (scene, _) = kashika_model::compile(&replaced);
    assert!(scene.lines.iter().any(|l| l.id == "line@L6"));
}

#[test]
fn existing_lines_between_the_same_shapes_are_listed() {
    let src = format!(
        "{WALL}line ab a.top -> y 20 -> hv b.top\n\
line ba b.bottom -> y 220 -> hv a.bottom\n\
line aw a.right -> wall.left\n"
    );
    let s = suggest(&req(&src, "a.right", "b.left", Style::Corner)).unwrap();
    assert_eq!(s.existing_lines, ["ab", "ba"]);
    let s = reroute(&src, "ab", Style::Corner).unwrap();
    assert!(s.existing_lines.is_empty());
}

#[test]
fn mixed_points_from_the_command_line_keep_their_text() {
    let s = suggest(&req(DIAG, "(a.right.x, 100)", "b.left", Style::Corner)).unwrap();
    assert!(!s.candidates.is_empty());
    for c in &s.candidates {
        assert!(
            c.statement.starts_with("line p_b (a.right.x, 100) -> "),
            "{}",
            c.statement
        );
    }
    agrees_with_check(DIAG, &s);
}

#[test]
fn control_points_follow_an_endpoint_written_with_spaces() {
    let s = suggest(&req(DIAG, "a.right + (0, 4)", "b.left", Style::Bezier)).unwrap();
    assert_eq!(
        s.candidates[0].statement,
        "line a_b a.right + (0, 4) -> curve via a.right+(75,4) b.left+(-75,0) b.left end=arrow"
    );
    agrees_with_check(DIAG, &s);
}

#[test]
fn probes_carry_the_rerouted_line_attributes() {
    let line = "line ab a.top -> b.top start=dot end=arrow label=\"x\" radius=4";
    let src = format!("{WALL}{line}\nline blocker (160,10) -> (160,230)\n");
    for style in [Style::Corner, Style::Arc] {
        let s = suggest(&Request {
            src: &src,
            from: None,
            to: None,
            style,
            line: Some("ab"),
            radius: 6.0,
            max: 10,
            opts: &OPTS,
        })
        .unwrap();
        assert!(!s.candidates.is_empty());
        for c in &s.candidates {
            assert!(
                c.statement.contains("start=dot end=arrow label=\"x\""),
                "{}",
                c.statement
            );
            let (scene, diags) = kashika_model::compile(&src.replace(line, &c.statement));
            assert!(diags.is_empty(), "{}: {diags:#?}", c.statement);
            let mine: Vec<_> = kashika_check::check(&scene)
                .into_iter()
                .filter(|d| d.elements.iter().any(|e| e.id == "ab"))
                .collect();
            let count = |codes: &[&str]| mine.iter().filter(|d| codes.contains(&d.code)).count();
            assert_eq!(count(&["cross"]), c.crossings, "{}", c.statement);
            assert_eq!(
                count(&["near", "shared-segment"]),
                c.near,
                "{}",
                c.statement
            );
        }
    }
}

#[test]
fn keeps_only_the_allow_entries_each_suggestion_needs() {
    let line = "line ab a.right -> b.left end=arrow allow=cross:l_feat,overlap reason=\"bus\"";
    let src = format!(
        "diagram \"r\" size (460,240)\n\
box a \"A\" at (20,100) size (100,40)\n\
box b \"B\" at (320,100) size (100,40)\n\
line l_feat (220,60) -> (220,180)\n\
{line}\n"
    );
    let s = suggest(&Request {
        max: 30,
        ..reroute_req(&src, "ab")
    })
    .unwrap();
    let clear = &s.candidates[0];
    assert_eq!(clear.crossings, 0);
    assert_eq!(clear.dropped_allow, ["cross:l_feat", "overlap"]);
    assert!(
        clear.statement.ends_with(" end=arrow"),
        "{}",
        clear.statement
    );
    let crossing = s
        .candidates
        .iter()
        .find(|c| c.crossings == 1)
        .expect("the straight path is a candidate");
    assert_eq!(
        crossing.statement,
        "line ab a.right -> b.left end=arrow allow=cross:l_feat reason=\"bus\""
    );
    assert_eq!(crossing.dropped_allow, ["overlap"]);
    for c in &s.candidates {
        let (scene, diags) = kashika_model::compile(&src.replace(line, &c.statement));
        assert!(diags.is_empty(), "{}: {diags:#?}", c.statement);
        assert!(
            kashika_check::check(&scene)
                .iter()
                .filter(|d| d.code == "cross")
                .all(|d| d.suppressed),
            "{}",
            c.statement
        );
    }
}

#[test]
fn keeps_reason_without_allow() {
    let src = format!("{WALL}line ab a.top -> y 28 -> hv b.top reason=\"x\"\n");
    let s = reroute(&src, "ab", Style::Corner).unwrap();
    assert!(s.candidates[0].statement.ends_with(" reason=\"x\""));
    assert!(s.candidates[0].dropped_allow.is_empty());
}

#[test]
fn ties_prefer_lanes_on_the_endpoint_boxes() {
    let src = "diagram \"t\" size (500,200)\n\
box a at (20,60) size (100,60)\n\
box b at (200,60) size (100,60)\n\
box c at (380,60) size (100,60)\n";
    let mut r = req(src, "a.right", "c.left", Style::Corner);
    r.max = 10;
    let s = suggest(&r).unwrap();
    let on_b = |c: &kashika_route::Candidate| c.statement.contains(" b.");
    let score = |c: &kashika_route::Candidate| (c.crossings, c.near, c.segments, c.length);
    assert!(
        s.candidates.iter().any(on_b),
        "a lane on b ties with lanes on a and c"
    );
    for (i, early) in s.candidates.iter().enumerate() {
        for late in &s.candidates[i + 1..] {
            assert!(
                !(score(early) == score(late) && on_b(early) && !on_b(late)),
                "{} ranks before the equally scored {}",
                early.statement,
                late.statement
            );
        }
    }
}
