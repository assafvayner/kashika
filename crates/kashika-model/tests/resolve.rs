use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, Rect};
use kashika_model::{EdgeKind, Padding, PlacementEdge, Scene, SegmentKind, StrokeStyle, compile};

const HEAD: &str = "diagram \"t\" size (800,600)\n";

fn ok(body: &str) -> Scene {
    let (scene, diags) = compile(&format!("{HEAD}{body}"));
    assert!(diags.is_empty(), "{diags:#?}");
    scene
}

fn errors(body: &str) -> Vec<Diagnostic> {
    compile(&format!("{HEAD}{body}")).1
}

fn rect(s: &Scene, id: &str) -> Rect {
    s.shapes.iter().find(|x| x.id == id).unwrap().rect
}

fn pts(s: &Scene, id: &str) -> Vec<Pt> {
    let l = s.lines.iter().find(|l| l.id == id).unwrap();
    let mut v = vec![l.path[0].from()];
    v.extend(l.path.iter().map(PathSeg::to));
    v
}

#[test]
fn group_children_use_the_group_frame() {
    let s = ok("group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n}\n");
    assert_eq!(rect(&s, "a"), Rect::new(110.0, 70.0, 40.0, 30.0));
    assert_eq!(
        s.shapes
            .iter()
            .find(|x| x.id == "a")
            .unwrap()
            .parent
            .as_deref(),
        Some("g")
    );
}

#[test]
fn relative_placement_with_pin() {
    let s = ok(
        "box api at (40,40) size (120,60)\nbox db at api.right+(160,0) size (120,60) pin=left\n",
    );
    assert_eq!(rect(&s, "db"), Rect::new(320.0, 40.0, 120.0, 60.0));
}

#[test]
fn forward_references_resolve() {
    let s = ok(
        "box db at api.right+(160,0) size (120,60) pin=left\nbox api at (40,40) size (120,60)\n",
    );
    assert_eq!(rect(&s, "db"), Rect::new(320.0, 40.0, 120.0, 60.0));
}

#[test]
fn mixed_point_inside_a_group_frame() {
    let s = ok(
        "box a at (200,0) size (100,40)\ngroup g at (0,100) size (400,300) {\n  junction j at (a.left.x, 50)\n}\n",
    );
    assert_eq!(s.junctions[0].at, Pt::new(200.0, 150.0));
}

#[test]
fn groups_default_to_the_boundary_style() {
    let s = ok("group g at (0,0) size (10,10) {\n}\n");
    assert_eq!(s.shapes[0].style.stroke_style, StrokeStyle::Dashed);
}

#[test]
fn user_style_overrides_builtin_key_by_key() {
    let s = ok("style service { fill=#ffffff }\nbox a at (0,0) size (1,1) style=service\n");
    assert_eq!(s.shapes[0].style.fill, "#ffffff");
    assert_eq!(s.shapes[0].style.stroke, "#1a73e8");
}

#[test]
fn placement_cycle_is_reported_once() {
    let d = errors("box a at b.right size (10,10)\nbox b at a.left size (10,10)\n");
    let cycles: Vec<_> = d.iter().filter(|d| d.code == "placement-cycle").collect();
    assert_eq!(cycles.len(), 1, "{d:#?}");
    assert!(cycles[0].message.contains("a -> b -> a"));
}

#[test]
fn unknown_id_suggests() {
    let d = errors("box api at (0,0) size (10,10)\nline apo.left -> (50,50)\n");
    assert_eq!(d[0].code, "unknown-id");
    assert_eq!(d[0].help, vec!["did you mean `api`?".to_string()]);
}

#[test]
fn unknown_anchor_suggests() {
    let d = errors("box api at (0,0) size (10,10)\nline api.rigth -> (50,50)\n");
    assert_eq!(d[0].code, "unknown-anchor");
    assert_eq!(d[0].message, "unknown anchor `rigth` on `api`");
    assert_eq!(d[0].help, vec!["did you mean `right`?".to_string()]);
}

#[test]
fn junctions_have_no_anchors() {
    let d = errors("junction j at (0,0)\nline j.left -> (1,1)\n");
    assert_eq!(d[0].code, "unknown-anchor");
}

#[test]
fn boxes_need_an_anchor() {
    let d = errors("box a at (0,0) size (1,1)\nline a -> (1,1)\n");
    assert_eq!(d[0].code, "unknown-anchor");
    assert!(d[0].message.contains("needs an anchor"));
}

#[test]
fn duplicate_ids() {
    let d = errors("box a at (0,0) size (1,1)\nbox a at (5,5) size (1,1)\n");
    assert_eq!(d[0].code, "duplicate-id");
    assert!(d[0].message.contains("line 2"));
}

#[test]
fn unknown_attribute_suggests() {
    let d = errors("box a at (0,0) size (10,10) fil=#fff\n");
    assert_eq!(d[0].code, "attr");
    assert_eq!(d[0].help, vec!["did you mean `fill`?".to_string()]);
}

#[test]
fn unknown_style_suggests() {
    let d = errors("box a at (0,0) size (10,10) style=servce\n");
    assert_eq!(d[0].code, "unknown-style");
    assert_eq!(d[0].help, vec!["did you mean `service`?".to_string()]);
}

#[test]
fn missing_diagram() {
    let (_, d) = compile("box a at (0,0) size (1,1)\n");
    assert_eq!(d[0].code, "diagram");
}

fn version_diags(attr: &str) -> Vec<Diagnostic> {
    compile(&format!("diagram \"t\" size (1,1) kashika={attr}\n")).1
}

#[test]
fn version_accepts_equal_and_older() {
    for v in ["\"0.1.0\"", "\"0.0.5\""] {
        assert!(version_diags(v).is_empty(), "{v}");
    }
}

#[test]
fn version_is_optional() {
    assert!(compile("diagram \"t\" size (1,1)\n").1.is_empty());
}

#[test]
fn version_rejects_newer_and_other_major() {
    for v in ["\"0.2.0\"", "\"0.1.1\"", "\"1.0.0\""] {
        let d = version_diags(v);
        assert_eq!(d.len(), 1, "{v}");
        assert_eq!(d[0].code, "version");
        assert!(
            d[0].message
                .contains("or newer (same major version); this is kashika 0.1.0")
        );
        assert!(!d[0].help.is_empty());
    }
}

#[test]
fn version_rejects_bare_and_malformed() {
    for v in [
        "1",
        "2",
        "\"1\"",
        "\"0.1\"",
        "\"abc\"",
        "\"0.1.0-beta\"",
        "\"0.1.0+b1\"",
        "\"0.1.0.0\"",
    ] {
        let d = version_diags(v);
        assert_eq!(d.len(), 1, "{v}");
        assert_eq!(d[0].code, "version");
        assert_eq!(
            d[0].help,
            vec!["write kashika=\"0.1.0\"".to_string()],
            "{v}"
        );
    }
}

#[test]
fn allow_validates_codes_and_ids() {
    let d = errors("line l1 (0,0) -> h 5 allow=crosss\nline l2 (0,0) -> v 5 allow=cross:nope\n");
    assert!(
        d.iter()
            .any(|d| d.code == "attr" && d.help == vec!["did you mean `cross`?".to_string()])
    );
    assert!(d.iter().any(|d| d.code == "unknown-id"));
}

#[test]
fn shapes_report_their_rects_after_errors_elsewhere() {
    let (scene, d) = compile(&format!(
        "{HEAD}box a at (0,0) size (10,10)\nline nope.left -> (1,1)\n"
    ));
    assert_eq!(d.len(), 1);
    assert_eq!(scene.shapes.len(), 1);
    assert!(scene.lines.is_empty());
}

#[test]
fn rounded_shape_projects_anchors() {
    let s = ok("box r at (0,0) size (100,60) shape=rounded\nline l r.top@0 -> (0,-50)\n");
    assert_eq!(pts(&s, "l")[0], Pt::new(0.0, 8.0));
}

#[test]
fn every_segment_kind_resolves() {
    let s = ok(
        "box a at (0,0) size (100,60)\nbox b at (300,200) size (100,60)\n\
                line l a.right -> h 50 -> y b.top -> x 400 -> v 10 -> hv (500,300) -> vh (520,320)\n",
    );
    let p = |x: f64, y: f64| Pt::new(x, y);
    assert_eq!(
        pts(&s, "l"),
        vec![
            p(100.0, 30.0),
            p(150.0, 30.0),
            p(150.0, 200.0),
            p(400.0, 200.0),
            p(400.0, 210.0),
            p(500.0, 210.0),
            p(500.0, 300.0),
            p(500.0, 320.0),
            p(520.0, 320.0),
        ]
    );
}

#[test]
fn already_aligned_elbows_drop_zero_length_pieces() {
    let s = ok("line l (0,0) -> hv (100,0)\n");
    assert_eq!(s.lines[0].path.len(), 1);
}

#[test]
fn numeric_axis_targets_use_the_group_frame() {
    let s = ok("group g at (100,100) size (300,300) {\n  line l (0,0) -> x 50 -> y 20\n}\n");
    assert_eq!(
        pts(&s, "l"),
        vec![
            Pt::new(100.0, 100.0),
            Pt::new(150.0, 100.0),
            Pt::new(150.0, 120.0)
        ]
    );
}

#[test]
fn curve_left_bulges_up_when_travelling_right() {
    let s = ok("line l (0,100) -> curve left 20 (100,100)\n");
    let PathSeg::Quad { ctrl, .. } = s.lines[0].path[0] else {
        panic!("expected quad")
    };
    assert_eq!(ctrl, Pt::new(50.0, 60.0));
    assert_eq!(s.lines[0].path[0].eval(0.5), Pt::new(50.0, 80.0));
}

#[test]
fn curve_right_bulges_right_when_travelling_up() {
    let s = ok("line l (0,100) -> curve right 10 (0,0)\n");
    let PathSeg::Quad { ctrl, .. } = s.lines[0].path[0] else {
        panic!("expected quad")
    };
    assert_eq!(ctrl, Pt::new(20.0, 50.0));
}

#[test]
fn curve_via_makes_a_cubic() {
    let s = ok("line l (0,0) -> curve via (0,50) (100,50) (100,0)\n");
    assert!(matches!(s.lines[0].path[0], PathSeg::Cubic { .. }));
}

#[test]
fn radius_rounds_polyline_corners() {
    let s = ok("line l (0,0) -> h 100 -> v 100 radius=10\n");
    assert_eq!(s.lines[0].path.len(), 3);
    assert!(matches!(s.lines[0].path[1], PathSeg::Quad { .. }));
}

#[test]
fn labels_markers_and_unnamed_ids() {
    let s = ok("line (0,0) -> h 100 label=\"a\" label@0.25=\"b\" end=arrow start=dot\n");
    let l = &s.lines[0];
    assert_eq!(l.id, "line@L2");
    assert!(!l.named);
    assert_eq!(
        l.labels.iter().map(|x| x.t).collect::<Vec<_>>(),
        vec![0.5, 0.25]
    );
    assert_eq!(
        (l.start, l.end),
        (kashika_model::Marker::Dot, kashika_model::Marker::Arrow)
    );
}

#[test]
fn ortho_rejects_diagonals() {
    let d = errors("line l (0,0) -> (10,10) ortho\n");
    assert_eq!(d[0].code, "not-ortho");
}

#[test]
fn degenerate_curve() {
    let d = errors("line l (0,0) -> curve left 10 (0,0)\n");
    assert_eq!(d[0].code, "degenerate-curve");
}

#[test]
fn zero_length_line() {
    let d = errors("line l (0,0) -> h 0\n");
    assert_eq!(d[0].code, "zero-length");
}

fn codes(body: &str) -> Vec<String> {
    errors(body)
        .into_iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn failed_segment_stops_the_line() {
    assert_eq!(
        codes("line l (0,0) -> nope.left -> curve left 10 (0,0)\n"),
        vec!["unknown-id"]
    );
}

#[test]
fn non_finite_values_are_rejected() {
    let big = "9".repeat(400);
    for body in [
        format!("box a at ({big},0) size (1,1)\n"),
        format!("box a at (0,0) size ({big},1)\n"),
        format!("line l (0,0) -> h {big}\n"),
        format!("line l (0,0) -> v {big}\n"),
        format!("line l (0,0) -> curve left {big} (10,0)\n"),
        format!("line l (0,0) -> curve via ({big},0) (1,1) (2,2)\n"),
    ] {
        let d = errors(&body);
        assert!(
            d.iter()
                .any(|d| d.code == "range" && d.message == "value is not a finite number"),
            "{body}: {d:#?}"
        );
    }
    let (_, d) = compile(&format!("diagram \"t\" size ({big},1)\n"));
    assert!(
        d.iter()
            .any(|d| d.message == "value is not a finite number"),
        "{d:#?}"
    );
}

#[test]
fn diagram_size_must_be_positive() {
    let (_, d) = compile("diagram \"t\" size (0,10)\n");
    assert!(d.iter().any(|d| d.code == "size"), "{d:#?}");
}

#[test]
fn group_honors_user_boundary_style() {
    let s = ok("style boundary { stroke=#ff0000 }\ngroup g at (0,0) size (10,10) {\n}\n");
    assert_eq!(s.shapes[0].style.stroke, "#ff0000");
    assert_eq!(s.shapes[0].style.stroke_style, StrokeStyle::Dashed);
}

#[test]
fn duplicate_ids_are_not_emitted_twice() {
    let (s, d) = compile(&format!(
        "{HEAD}box a at (0,0) size (1,1)\ngroup a at (5,5) size (10,10) {{\n  box c at (0,0) size (1,1)\n}}\n"
    ));
    assert_eq!(d.iter().filter(|d| d.code == "duplicate-id").count(), 1);
    assert_eq!(s.shapes.iter().filter(|x| x.id == "a").count(), 1);
    assert!(s.shapes.iter().all(|x| x.id != "c"));
}

#[test]
fn duplicate_label_positions() {
    let d = errors("line l (0,0) -> h 10 label=\"a\" label@0.5=\"b\"\n");
    assert_eq!(
        d.iter()
            .filter(|d| d.code == "attr" && d.message.starts_with("duplicate label at t="))
            .count(),
        1
    );
    let (s, _) = compile(&format!(
        "{HEAD}line l (0,0) -> h 10 label@0.3=\"a\" label@0.3=\"b\"\n"
    ));
    assert_eq!(s.lines[0].labels.len(), 1);
    assert_eq!(s.lines[0].labels[0].text, "a");
}

#[test]
fn radius_makes_rect_outline_rounded() {
    let s = ok("box a at (0,0) size (100,60) radius=8\n");
    assert_eq!(s.shapes[0].outline, kashika_model::Outline::Rounded);
    let s = ok("box a at (0,0) size (100,60)\n");
    assert_eq!(s.shapes[0].outline, kashika_model::Outline::Rect);
}

#[test]
fn negative_curve_amount() {
    let d = errors("line l (0,0) -> curve left -10 (10,0)\n");
    assert_eq!(d[0].code, "degenerate-curve");
    assert_eq!(
        d[0].message,
        "curve amount must not be negative; use `left`/`right` to choose the side"
    );
}

#[test]
fn unknown_attributes_fail_clearly() {
    let d = errors("box a at (0,0) size (10,10) fil=#fff\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn duplicate_label_attribute_reports_once() {
    let d = errors("line l (0,0) -> h 5 label@0.3=\"a\" label@0.3=\"b\"\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn duplicate_icon_attribute_reports_once() {
    let d = errors("box a at (0,0) size (1,1) icon-size=4 icon-size=5\n");
    let codes: Vec<_> = d.iter().map(|x| x.code).collect();
    assert_eq!(codes.iter().filter(|c| **c == "attr").count(), 2, "{d:#?}");
}

#[test]
fn zero_curve_amount_is_allowed() {
    ok("line l (0,0) -> curve left 0 (10,0)\n");
}

#[test]
fn negative_curve_message() {
    let d = errors("line l (0,0) -> curve left -1 (10,0)\n");
    assert!(
        d.iter().any(|x| x.message.contains("must not be negative")),
        "{d:#?}"
    );
}

#[test]
fn children_of_duplicate_group_are_not_reference_targets() {
    let d = errors(
        "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  box b at (0,0) size (10,10)\n}\nline l (0,0) -> b.left\n",
    );
    assert_eq!(codes_of(&d), vec!["duplicate-id"], "{d:#?}");
}

#[test]
fn grandchildren_of_duplicate_group_are_not_reference_targets() {
    let d = errors(
        "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  group inner at (0,0) size (50,50) {\n    box b at (0,0) size (10,10)\n  }\n}\nline l (0,0) -> b.left\n",
    );
    assert_eq!(codes_of(&d), vec!["duplicate-id"], "{d:#?}");
    let d = errors(
        "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  box b at (0,0) size (10,10)\n}\nline l (0,0) -> h 5 allow=cross:b\nline m (0,0) -> bb.left\n",
    );
    assert!(d.iter().all(|d| !d.message.contains("`b`")), "{d:#?}");
    assert!(
        d.iter()
            .any(|d| d.code == "unknown-id" && d.message.contains("`bb`") && d.help.is_empty()),
        "skipped ids are not suggested: {d:#?}"
    );
}

fn junction(s: &Scene, id: &str) -> Pt {
    s.junctions.iter().find(|j| j.id == id).unwrap().at
}

#[test]
fn nested_groups_compose_frames() {
    let s = ok(
        "group outer at (100,100) size (400,400) {\n  group inner at (50,60) size (200,200) {\n    box a at (5,5) size (10,10)\n    line l (0,0) -> h 10\n  }\n}\n",
    );
    assert_eq!(rect(&s, "inner"), Rect::new(150.0, 160.0, 200.0, 200.0));
    assert_eq!(rect(&s, "a"), Rect::new(155.0, 165.0, 10.0, 10.0));
    assert_eq!(pts(&s, "l")[0], Pt::new(150.0, 160.0));
}

#[test]
fn anchor_references_inside_a_group_are_absolute() {
    let s = ok(
        "box a at (300,300) size (100,60)\ngroup g at (100,100) size (100,100) {\n  box b at a.right+(10,0) size (20,20) pin=left\n  junction j at a.tl\n  line l (0,0) -> a.left\n}\n",
    );
    assert_eq!(rect(&s, "b"), Rect::new(410.0, 320.0, 20.0, 20.0));
    assert_eq!(junction(&s, "j"), Pt::new(300.0, 300.0));
    assert_eq!(*pts(&s, "l").last().unwrap(), Pt::new(300.0, 330.0));
}

#[test]
fn references_into_groups_resolve_to_true_positions() {
    let s = ok(
        "group g at (100,100) size (300,300) {\n  box a at (10,10) size (40,20)\n}\nline l a.right -> h 50\n",
    );
    assert_eq!(pts(&s, "l")[0], Pt::new(150.0, 120.0));
}

#[test]
fn hv_and_curve_via_points_use_the_group_frame() {
    let s = ok(
        "group g at (100,100) size (300,300) {\n  line l (0,0) -> hv (50,50) -> curve via (60,50) (70,60) (70,70)\n}\n",
    );
    assert_eq!(
        pts(&s, "l"),
        vec![
            Pt::new(100.0, 100.0),
            Pt::new(150.0, 100.0),
            Pt::new(150.0, 150.0),
            Pt::new(170.0, 170.0)
        ]
    );
    let PathSeg::Cubic { c1, c2, .. } = s.lines[0].path[2] else {
        panic!("expected cubic")
    };
    assert_eq!((c1, c2), (Pt::new(160.0, 150.0), Pt::new(170.0, 160.0)));
}

#[test]
fn text_and_junction_literals_use_the_group_frame() {
    let s = ok(
        "group g at (100,50) size (300,300) {\n  junction j at (10,20)\n  text t \"hi\" at (30,40)\n}\n",
    );
    assert_eq!(junction(&s, "j"), Pt::new(110.0, 70.0));
    assert_eq!(s.texts[0].at, Pt::new(130.0, 90.0));
    assert_eq!(s.texts[0].parent.as_deref(), Some("g"));
}

#[test]
fn negative_moves_go_left_and_up() {
    let s = ok("line l (100,100) -> h -30 -> v -40\n");
    assert_eq!(
        pts(&s, "l"),
        vec![
            Pt::new(100.0, 100.0),
            Pt::new(70.0, 100.0),
            Pt::new(70.0, 60.0)
        ]
    );
}

#[test]
fn axis_targets_take_a_points_coordinate() {
    let s =
        ok("box a at (200,300) size (100,60)\nline l (0,0) -> x a.right -> y a.bottom+(0,10)\n");
    assert_eq!(
        pts(&s, "l"),
        vec![
            Pt::new(0.0, 0.0),
            Pt::new(300.0, 0.0),
            Pt::new(300.0, 370.0)
        ]
    );
}

#[test]
fn every_anchor_and_alias_resolves() {
    let s = ok("box a at (100,100) size (100,60)\n");
    let r = rect(&s, "a");
    for (name, want) in [
        ("top", (150.0, 100.0)),
        ("bottom", (150.0, 160.0)),
        ("left", (100.0, 130.0)),
        ("right", (200.0, 130.0)),
        ("center", (150.0, 130.0)),
        ("tl", (100.0, 100.0)),
        ("tr", (200.0, 100.0)),
        ("bl", (100.0, 160.0)),
        ("br", (200.0, 160.0)),
        ("top-left", (100.0, 100.0)),
        ("top-right", (200.0, 100.0)),
        ("bottom-left", (100.0, 160.0)),
        ("bottom-right", (200.0, 160.0)),
        ("top@0.5", (150.0, 100.0)),
        ("top@0", (100.0, 100.0)),
        ("bottom@0.25", (125.0, 160.0)),
        ("left@0.25", (100.0, 115.0)),
        ("right@1", (200.0, 160.0)),
    ] {
        let s = ok(&format!(
            "box a at ({},{}) size ({},{})\njunction j at a.{name}\n",
            r.x, r.y, r.w, r.h
        ));
        assert_eq!(junction(&s, "j"), Pt::new(want.0, want.1), "{name}");
    }
}

#[test]
fn anchor_t_and_offset_compose() {
    let s = ok(
        "box db at (0,0) size (100,40)\njunction j at db.left@0.25+(0,4)\njunction k at db.top+(-10,0)\n",
    );
    assert_eq!(junction(&s, "j"), Pt::new(0.0, 14.0));
    assert_eq!(junction(&s, "k"), Pt::new(40.0, 0.0));
}

#[test]
fn group_anchors_work_like_box_anchors() {
    let s = ok(
        "group k8s at (100,100) size (200,100) {\n}\njunction j at k8s.top\nbox b at k8s.right+(20,0) size (10,10) pin=left\n",
    );
    assert_eq!(junction(&s, "j"), Pt::new(200.0, 100.0));
    assert_eq!(rect(&s, "b"), Rect::new(320.0, 145.0, 10.0, 10.0));
}

#[test]
fn junction_references_take_offsets_and_axes() {
    let s = ok(
        "junction j at (40,50)\njunction k at j+(5,-5)\njunction m at (j.x, k.y)\nline l j -> k\n",
    );
    assert_eq!(junction(&s, "k"), Pt::new(45.0, 45.0));
    assert_eq!(junction(&s, "m"), Pt::new(40.0, 45.0));
    assert_eq!(pts(&s, "l"), vec![Pt::new(40.0, 50.0), Pt::new(45.0, 45.0)]);
}

#[test]
fn mixed_points_take_each_axis_from_a_reference() {
    let s = ok(
        "box app1 at (100,0) size (60,40)\njunction lane1 at (0,200)\nline l app1.bottom -> (app1.bottom.x, lane1.y)\n",
    );
    assert_eq!(
        pts(&s, "l"),
        vec![Pt::new(130.0, 40.0), Pt::new(130.0, 200.0)]
    );
}

#[test]
fn anchor_position_out_of_range() {
    let d = errors("box a at (0,0) size (10,10)\njunction j at a.top@1.5\n");
    assert_eq!(d[0].code, "unknown-anchor");
    assert_eq!(d[0].message, "`t` must be between 0 and 1");
    assert_eq!(d[0].help.len(), 1, "{d:#?}");
    for corner in ["tl", "center", "bottom-right"] {
        let d = errors(&format!(
            "box a at (0,0) size (10,10)\njunction j at a.{corner}@0.5\n"
        ));
        assert_eq!(d[0].code, "unknown-anchor");
        assert_eq!(
            d[0].message,
            "`@t` only applies to `top`/`bottom`/`left`/`right`"
        );
        assert_eq!(d[0].help.len(), 1, "{d:#?}");
    }
}

#[test]
fn every_pin_anchor_lands_on_at() {
    for (pin, want) in [
        ("tl", (100.0, 100.0)),
        ("top", (80.0, 100.0)),
        ("tr", (60.0, 100.0)),
        ("right", (60.0, 90.0)),
        ("br", (60.0, 80.0)),
        ("bottom", (80.0, 80.0)),
        ("bl", (100.0, 80.0)),
        ("left", (100.0, 90.0)),
        ("center", (80.0, 90.0)),
        ("top-left", (100.0, 100.0)),
        ("bottom-right", (60.0, 80.0)),
    ] {
        let s = ok(&format!("box a at (100,100) size (40,20) pin={pin}\n"));
        assert_eq!(
            rect(&s, "a"),
            Rect::new(want.0, want.1, 40.0, 20.0),
            "{pin}"
        );
    }
}

#[test]
fn pin_rejects_unknown_anchors_and_positions() {
    let d = errors("box a at (0,0) size (10,10) pin=lft\n");
    assert_eq!(d[0].code, "attr");
    assert_eq!(d[0].help, vec!["did you mean `left`?".to_string()]);
}

#[test]
fn pin_accepts_a_position_on_an_edge() {
    let s = ok(
        "box api at (40,40) size (120,60)\nbox db at api.right+(100,0) size (100,80) pin=left@0.25\n",
    );
    assert_eq!(rect(&s, "db"), Rect::new(260.0, 50.0, 100.0, 80.0));
}

#[test]
fn pin_position_errors() {
    assert_eq!(
        errors("box a at (0,0) size (10,10) pin=tl@0.5\n")[0].code,
        "unknown-anchor"
    );
    assert_eq!(
        errors("box a at (0,0) size (10,10) pin=left@2\n")[0].code,
        "unknown-anchor"
    );
    assert_eq!(
        errors("box a at (0,0) size (10,10) pin=lft@0.5\n")[0].code,
        "attr"
    );
}

#[test]
fn placement_cycle_points_at_the_at_clause() {
    let d = errors("box a at b.right size (10,10)\nbox b at a.left size (10,10)\n");
    let c = d.iter().find(|d| d.code == "placement-cycle").unwrap();
    assert_eq!((c.span.line, c.span.col), (2, 10));
}

#[test]
fn duplicate_style_has_its_own_code() {
    let d = errors("style s { fill=#fff }\nstyle s { fill=#000 }\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "duplicate-style");
    assert_eq!(d[0].span.line, 3);
    ok("style a { fill=#fff }\nbox a at (0,0) size (1,1) style=a\n");
}

#[test]
fn statements_after_unbuilt_ones_still_resolve() {
    let (s, d) = kashika_model::compile(&format!(
        "{HEAD}import \"lib.kash\"\nshape s {{\n  path \"M 0 0\"\n}}\nimage i \"x.png\" at (0,0) size (1,1)\n\
         box a at (10,10) size (20,20) padding=0 {{\n  text \"x\"\n}}\nbox b at a.right+(10,0) size (20,20) pin=left\nline l a.right -> b.left\n"
    ));
    let codes: Vec<_> = d.iter().map(|x| x.code).collect();
    assert_eq!(codes, vec!["file", "parse", "file"], "{d:#?}");
    assert_eq!(rect(&s, "b"), Rect::new(40.0, 10.0, 20.0, 20.0));
    assert_eq!(pts(&s, "l"), vec![Pt::new(30.0, 20.0), Pt::new(40.0, 20.0)]);
}

#[test]
fn group_pin_moves_its_frame() {
    let s = ok("group g at (100,100) size (40,20) pin=center {\n  box a at (0,0) size (1,1)\n}\n");
    assert_eq!(rect(&s, "g"), Rect::new(80.0, 90.0, 40.0, 20.0));
    assert_eq!(rect(&s, "a"), Rect::new(80.0, 90.0, 1.0, 1.0));
}

#[test]
fn self_placement_is_a_cycle() {
    let d = errors("box a at a.left size (10,10)\n");
    assert_eq!(d[0].code, "placement-cycle");
    assert!(d[0].message.contains("a -> a"), "{d:#?}");
}

#[test]
fn group_placed_relative_to_its_own_child_is_a_cycle() {
    let d = errors("group g at c.left size (100,100) {\n  box c at (0,0) size (10,10)\n}\n");
    assert!(d.iter().any(|x| x.code == "placement-cycle"), "{d:#?}");
}

#[test]
fn mixed_point_cycles_are_detected() {
    let d = errors("junction j at (k.x, 0)\njunction k at (j.x, 0)\n");
    assert_eq!(
        d.iter().filter(|x| x.code == "placement-cycle").count(),
        1,
        "{d:#?}"
    );
}

#[test]
fn ids_are_unique_across_kinds_and_groups() {
    for body in [
        "box a at (0,0) size (1,1)\nline a (0,0) -> h 1\n",
        "junction a at (0,0)\ntext a \"x\" at (0,0)\n",
        "box a at (0,0) size (1,1)\ngroup g at (0,0) size (10,10) {\n  junction a at (0,0)\n}\n",
    ] {
        let d = errors(body);
        assert_eq!(d.len(), 1, "{body}: {d:#?}");
        assert_eq!(d[0].code, "duplicate-id", "{body}");
    }
}

#[test]
fn texts_and_lines_cannot_be_referenced() {
    let d = errors("text t \"x\" at (0,0)\njunction j at t.left\n");
    assert_eq!(d[0].code, "unknown-id");
}

#[test]
fn inline_attributes_override_named_style() {
    let s = ok(
        "style svc { fill=#111111 stroke=#222222 }\nbox a at (0,0) size (1,1) style=svc fill=#333333\n",
    );
    assert_eq!(s.shapes[0].style.fill, "#333333");
    assert_eq!(s.shapes[0].style.stroke, "#222222");
}

#[test]
fn group_style_and_inline_apply_over_boundary() {
    let s = ok(
        "style blue { stroke=#0000ff }\ngroup g at (0,0) size (10,10) style=blue stroke-style=solid {\n}\n",
    );
    assert_eq!(s.shapes[0].style.stroke, "#0000ff");
    assert_eq!(s.shapes[0].style.stroke_style, StrokeStyle::Solid);
}

#[test]
fn styles_may_be_defined_after_use() {
    let s = ok("box a at (0,0) size (1,1) style=mine\nstyle mine { fill=#abcdef }\n");
    assert_eq!(s.shapes[0].style.fill, "#abcdef");
}

#[test]
fn every_style_key_is_accepted() {
    ok(
        "style s { fill=#fff stroke=#000 stroke-width=2 stroke-style=dotted font-size=12 font-weight=bold color=#111 radius=4 padding=6 muted=#888 }\n",
    );
    let d = errors("style s { fil=#fff }\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn padding_pair_follows_style_precedence() {
    let s = ok("style s { padding=(2,4) }\n\
         box a at (0,0) size (10,10) style=s\n\
         box b at (0,20) size (10,10) style=s padding=6\n\
         box c at (0,40) size (10,10) style=s padding=(1,0)\n\
         box d at (0,60) size (10,10) style=service\n");
    let pad = |i: usize| s.shapes[i].style.padding;
    assert_eq!(pad(0), Padding { x: 2.0, y: 4.0 });
    assert_eq!(pad(1), Padding::uniform(6.0));
    assert_eq!(pad(2), Padding { x: 1.0, y: 0.0 });
    assert_eq!(pad(3), Padding::uniform(8.0));
}

#[test]
fn padding_pair_errors() {
    for (body, msg) in [
        (
            "box a at (0,0) size (10,10) padding=(-1,2)\n",
            "`padding` values must be at least 0 and at most 1000000",
        ),
        (
            "box a at (0,0) size (10,10) padding=(1,2000000)\n",
            "`padding` values must be at least 0 and at most 1000000",
        ),
        (
            "style s { padding=(0,-3) }\n",
            "`padding` values must be at least 0 and at most 1000000",
        ),
        (
            "box a at (0,0) size (10,10) padding=wide\n",
            "`padding` expects a number or a pair `(x,y)`",
        ),
        (
            "box a at (0,0) size (10,10) padding=\"8\"\n",
            "`padding` expects a number or a pair `(x,y)`",
        ),
    ] {
        let d = errors(body);
        assert_eq!(codes_of(&d), vec!["attr"], "{body}: {d:#?}");
        assert_eq!(d[0].message, msg);
        assert_eq!(d[0].help.len(), 1);
    }
}

#[test]
fn every_builtin_style_exists() {
    for name in [
        "service",
        "datastore",
        "external",
        "queue",
        "boundary",
        "note",
    ] {
        ok(&format!("box a at (0,0) size (1,1) style={name}\n"));
    }
}

#[test]
fn line_style_applies_only_stroke_and_font_keys() {
    let s = ok(
        "style s { fill=#ff0000 stroke=#00ff00 radius=10 font-size=20 }\nline l (0,0) -> h 100 -> v 100 style=s\n",
    );
    let l = &s.lines[0];
    assert_eq!(l.style.stroke, "#00ff00");
    assert_eq!(l.style.font_size, 20.0);
    assert_ne!(l.style.fill, "#ff0000");
    assert_eq!(l.path.len(), 2, "style radius must not round lines");
}

#[test]
fn line_inline_stroke_attributes() {
    let s = ok("line l (0,0) -> h 10 stroke=#123456 stroke-width=3 stroke-style=dashed\n");
    let st = &s.lines[0].style;
    assert_eq!(
        (st.stroke.as_str(), st.stroke_width, st.stroke_style),
        ("#123456", 3.0, StrokeStyle::Dashed)
    );
    assert_eq!(errors("line l (0,0) -> h 10 fill=#fff\n")[0].code, "attr");
}

#[test]
fn markers_default_to_none_and_accept_every_kind() {
    let s = ok("line l (0,0) -> h 10\n");
    assert_eq!(
        (s.lines[0].start, s.lines[0].end),
        (kashika_model::Marker::None, kashika_model::Marker::None)
    );
    for m in ["arrow", "open", "diamond", "dot", "none"] {
        ok(&format!("line l (0,0) -> h 10 start={m} end={m}\n"));
    }
    assert_eq!(errors("line l (0,0) -> h 10 end=arow\n")[0].code, "attr");
}

#[test]
fn label_position_must_be_in_range() {
    let d = errors("line l (0,0) -> h 10 label@1.5=\"x\"\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn ortho_rejects_curves() {
    let d = errors("line l (0,0) -> curve left 10 (100,0) ortho\n");
    assert_eq!(d[0].code, "not-ortho");
    ok("line l (0,0) -> h 10 -> vh (20,20) ortho\n");
}

#[test]
fn reason_is_kept() {
    let s = ok("line l (0,0) -> h 10 allow=cross reason=\"bus\"\n");
    assert_eq!(s.lines[0].reason.as_deref(), Some("bus"));
}

#[test]
fn unnamed_continued_line_is_named_after_its_first_line() {
    let s = ok("\nline (0,0)\n  -> h 10\n  -> v 10\n");
    assert_eq!(s.lines[0].id, "line@L3");
}

#[test]
fn diagram_attributes() {
    let s = ok("");
    assert_eq!((s.clearance, s.grid), (6.0, None));
    let (s, d) =
        compile("diagram \"t\" size (10,10) kashika=\"0.1.0\" grid=10 clearance=4 allow=near\n");
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!((s.clearance, s.grid, s.allow.len()), (4.0, Some(10.0), 1));
}

#[test]
fn diagram_background() {
    assert_eq!(ok("").background, None);
    let bg = |v: &str| compile(&format!("diagram \"t\" size (10,10) background={v}\n"));
    let (s, d) = bg("#0f172a");
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(s.background.as_deref(), Some("#0f172a"));
    let (s, d) = bg("none");
    assert!(d.is_empty(), "{d:#?}");
    assert_eq!(s.background, None);
    for v in ["reddish", "\"#ffffff\"", "3"] {
        let (s, d) = bg(v);
        assert_eq!(codes_of(&d), vec!["attr"], "{v}: {d:#?}");
        assert_eq!(s.background, None);
    }
}

#[test]
fn diagram_must_be_first_and_unique() {
    let (_, d) = compile("box a at (0,0) size (1,1)\ndiagram \"t\" size (10,10)\n");
    assert_eq!(d[0].code, "diagram");
    let (_, d) = compile("diagram \"t\" size (10,10)\ndiagram \"u\" size (10,10)\n");
    assert_eq!(d[0].code, "diagram");
    let (_, d) = compile("style s { fill=#fff }\ndiagram \"t\" size (10,10)\n");
    assert_eq!(d[0].code, "diagram");
}

#[test]
fn diagram_inside_a_group_is_misplaced() {
    let d = errors("group g at (0,0) size (10,10) {\n  diagram \"x\" size (1,1)\n}\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "diagram");
}

#[test]
fn pin_at_t_on_a_diamond_follows_the_outline() {
    let s = ok("box d at (200,100) size (100,80) shape=diamond pin=top@0.25\n");
    let d = &s.shapes[0];
    assert_eq!(d.rect, Rect::new(175.0, 80.0, 100.0, 80.0));
    assert_eq!(d.anchor("top", Some(0.25)), Some(Pt::new(200.0, 100.0)));
}

#[test]
fn shapes_need_positive_sizes() {
    for body in [
        "box a at (0,0) size (0,10)\n",
        "box a at (0,0) size (10,-1)\n",
        "group g at (0,0) size (0,0) {\n}\n",
        "text \"x\" at (0,0) size=0\n",
    ] {
        let d = errors(body);
        assert!(d.iter().any(|x| x.code == "size"), "{body}: {d:#?}");
    }
}

#[test]
fn box_shapes() {
    let s = ok("box a \"A\" at (0,0) size (40,40) shape=rect\n");
    assert_eq!(s.shapes[0].outline, kashika_model::Outline::Rect);
    assert_eq!(s.shapes[0].label.as_deref(), Some("A"));
    let s = ok("box a at (0,0) size (10,10) shape=rounded\n");
    assert_eq!(s.shapes[0].outline, kashika_model::Outline::Rounded);
    let d = errors("box a at (0,0) size (10,10) shape=rectt\n");
    assert_eq!(d[0].code, "unknown-shape");
}

#[test]
fn groups_take_no_shape() {
    let d = errors("group g at (0,0) size (10,10) shape=rect {\n}\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn junction_dot() {
    let s = ok("junction a at (0,0) dot\njunction b at (1,1)\n");
    assert_eq!(
        s.junctions.iter().map(|j| j.dot).collect::<Vec<_>>(),
        vec![true, false]
    );
    assert_eq!(errors("junction a at (0,0) dot=1\n")[0].code, "attr");
}

#[test]
fn text_attributes() {
    let s =
        ok("text t \"hi\" at (10,20) size=18 color=#ff0000 align=right\ntext \"anon\" at (0,0)\n");
    let t = &s.texts[0];
    assert_eq!(t.id.as_deref(), Some("t"));
    assert_eq!(
        (t.size, t.color.as_str(), t.align),
        (18.0, "#ff0000", kashika_model::Align::Right)
    );
    assert_eq!(s.texts[1].id, None);
    assert_eq!(s.texts[1].align, kashika_model::Align::Center);
    assert_eq!(errors("text \"x\" at (0,0) align=middle\n")[0].code, "attr");
}

#[test]
fn duplicate_and_mistyped_attributes() {
    let d = errors("box a at (0,0) size (10,10) fill=#fff fill=#000\n");
    assert_eq!(d[0].code, "attr");
    assert!(d[0].message.contains("duplicate"));
    let d = errors("box a at (0,0) size (10,10) stroke-width=\"2\"\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn style_inside_a_group_is_rejected() {
    let d = errors("group g at (0,0) size (10,10) {\n  style s { fill=#fff }\n}\n");
    assert_eq!(d[0].code, "parse");
}

fn codes_of(d: &[Diagnostic]) -> Vec<&'static str> {
    d.iter().map(|x| x.code).collect()
}

#[test]
fn out_of_range_values_are_rejected() {
    for body in [
        "box a at (1000001,0) size (1,1)\n",
        "box a at (0,-1000001) size (1,1)\n",
        "box a at (999999,0) size (1,1)\nbox b at a.right+(10,0) size (1,1)\n",
        "line l (0,0) -> h 2000000\n",
        "line l (0,0) -> x 2000000\n",
        "line l (0,0) -> curve left 2000000 (10,0)\n",
        "line l (0,0) -> curve via (1e7,0) (1,1) (2,2)\n",
    ] {
        let body = body.replace("1e7", "10000000");
        let d = errors(&body);
        assert!(
            d.iter().any(|d| d.code == "range"
                && d.message == "value is out of range (more than 1000000 from the origin)"),
            "{body}: {d:#?}"
        );
    }
    for body in [
        "box a at (0,0) size (1000001,1)\n",
        "group g at (0,0) size (1,2000000) {\n}\n",
    ] {
        let d = errors(body);
        assert_eq!(codes_of(&d), vec!["size"], "{body}: {d:#?}");
    }
    let (s, d) = compile("diagram \"t\" size (2000000,10)\n");
    assert_eq!(codes_of(&d), vec!["size"], "{d:#?}");
    assert_eq!(s.size, (0.0, 0.0));
    ok("box a at (-1000000,1000000) size (1000000,1)\n");
}

#[test]
fn long_placement_chains_are_reported_not_crashed() {
    let n = 5_000;
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!(
            "box b{i} at b{}.right size (1,1) pin=left\n",
            i + 1
        ));
    }
    body.push_str(&format!("box b{n} at (0,0) size (1,1)\n"));
    let d = errors(&body);
    assert_eq!(
        codes_of(&d),
        vec!["placement-cycle"],
        "{:#?}",
        &d[..d.len().min(3)]
    );
    assert_eq!(
        d[0].message,
        "placement chain from `b0` to `b5000` is 5000 references deep; the limit is 256"
    );

    let mut body = String::new();
    for i in 0..200 {
        body.push_str(&format!(
            "box b{i} at b{}.right size (1,1) pin=left\n",
            i + 1
        ));
    }
    body.push_str("box b200 at (0,0) size (1,1)\n");
    let s = ok(&body);
    assert_eq!(rect(&s, "b0").x, 200.0);
}

#[test]
fn markup_and_escapes_are_accepted_in_texts_and_labels() {
    let s = ok(
        "box a \"a \\*\\* b\" at (0,0) size (60,40)\ntext \"**t** \\*\" at (0,0)\nline l (0,0) -> h 5 label=\"`q`\"\n",
    );
    assert_eq!(s.shapes[0].label.as_deref(), Some("a \\*\\* b"));
    assert_eq!(s.texts[0].content, "**t** \\*");
    assert_eq!(s.lines[0].labels[0].text, "`q`");
    let (s, _) = compile("diagram \"a \\* b\" size (10,10)\n");
    assert_eq!(s.title, "a * b");
}

#[test]
fn missing_diagram_in_an_empty_file_has_a_real_span() {
    let (_, d) = compile("");
    assert_eq!(codes_of(&d), vec!["diagram"]);
    assert_eq!((d[0].span.line, d[0].span.col), (1, 1));
}

#[test]
fn numeric_attributes_are_range_checked() {
    for body in [
        "box a at (0,0) size (10,10) font-size=0\n",
        "box a at (0,0) size (10,10) stroke-width=-1\n",
        "box a at (0,0) size (10,10) padding=-1\n",
        "box a at (0,0) size (10,10) radius=-2\n",
        "style s { font-size=-3 }\n",
        "line l (0,0) -> h 5 stroke-width=0\n",
        "box a at (0,0) size (10,10) font-size=2000000\n",
    ] {
        let d = errors(body);
        assert_eq!(codes_of(&d), vec!["attr"], "{body}: {d:#?}");
    }
    for head in [
        "diagram \"t\" size (10,10) grid=0\n",
        "diagram \"t\" size (10,10) grid=-5\n",
        "diagram \"t\" size (10,10) clearance=-1\n",
    ] {
        let (_, d) = compile(head);
        assert_eq!(codes_of(&d), vec!["attr"], "{head}: {d:#?}");
    }
    let (_, d) = compile(
        "diagram \"t\" size (10,10) grid=10 clearance=0\nbox a at (0,0) size (10,10) padding=0 radius=0 stroke-width=0.5\n",
    );
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn boxes_with_a_bad_size_are_not_placed() {
    let (s, d) = compile(&format!(
        "{HEAD}box a at (0,0) size (0,10)\nbox b at a.right size (10,10)\ngroup g at (0,0) size (-1,10) {{\n  box c at (0,0) size (1,1)\n}}\n"
    ));
    assert_eq!(codes_of(&d), vec!["size", "size"], "{d:#?}");
    assert!(s.shapes.is_empty(), "{:#?}", s.shapes);
}

#[test]
fn diagram_must_come_first_even_after_a_broken_statement() {
    for src in [
        "bogus\ndiagram \"t\" size (10,10)\n",
        "box a at\ndiagram \"t\" size (10,10)\n",
        "import \"x.kash\"\ndiagram \"t\" size (10,10)\n",
    ] {
        let (_, d) = compile(src);
        assert!(
            d.iter()
                .any(|d| d.code == "diagram" && d.message.contains("first")),
            "{src}: {d:#?}"
        );
    }
    let (_, d) = compile("// c\n\n/* b */\ndiagram \"t\" size (10,10)\n");
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn text_color_cannot_be_none() {
    let d = errors("text \"t\" at (0,0) color=none\n");
    assert_eq!(codes_of(&d), vec!["attr"], "{d:#?}");
}

#[test]
fn elements_keep_their_frame_origin() {
    let s = ok(
        "group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n  junction j at (5,5)\n  text \"t\" at (1,2)\n  line l (0,0) -> h 10\n}\n",
    );
    let a = s.shapes.iter().find(|x| x.id == "a").unwrap();
    assert_eq!(a.origin, Pt::new(100.0, 50.0));
    assert_eq!(a.local_rect(), Rect::new(10.0, 20.0, 40.0, 30.0));
    assert_eq!(a.anchor("right", None), Some(Pt::new(150.0, 85.0)));
    assert_eq!(a.anchor("nope", None), None);
    let g = s.shapes.iter().find(|x| x.id == "g").unwrap();
    assert_eq!(g.origin, Pt::new(0.0, 0.0));
    assert_eq!(s.junctions[0].origin, Pt::new(100.0, 50.0));
    assert_eq!(s.texts[0].origin, Pt::new(100.0, 50.0));
    assert_eq!(s.lines[0].origin, Pt::new(100.0, 50.0));
}

#[test]
fn placement_graph_records_at_references_and_frames() {
    let s = ok(
        "box api at (40,40) size (120,60)\ngroup g at (0,200) size (400,200) {\n  box db at (api.left.x, 20) size (100,60)\n}\njunction j at db.bottom+(0,20)\n",
    );
    let e = |f: &str, t: &str, kind: EdgeKind| PlacementEdge {
        from: f.into(),
        to: t.into(),
        kind,
    };
    assert_eq!(
        s.placement,
        vec![
            e("db", "g", EdgeKind::Frame),
            e("db", "api", EdgeKind::At),
            e("j", "db", EdgeKind::At),
        ]
    );
}

#[test]
fn placement_graph_survives_cycles() {
    let (s, _) = compile(&format!(
        "{HEAD}box a at b.right size (10,10)\nbox b at a.left size (10,10)\n"
    ));
    assert_eq!(s.placement.len(), 2);
}

#[test]
fn lines_keep_their_authored_segments() {
    let s =
        ok("box a at (0,0) size (40,40)\nline l a.right -> h 20 -> hv (200,100) -> v 0 radius=5\n");
    let l = &s.lines[0];
    let kinds: Vec<SegmentKind> = l.segments.iter().map(|g| g.kind).collect();
    assert_eq!(kinds, [SegmentKind::H, SegmentKind::Hv, SegmentKind::V]);
    let hv = &l.segments[1];
    assert_eq!(
        (hv.from, hv.to),
        (Pt::new(60.0, 20.0), Pt::new(200.0, 100.0))
    );
    assert_eq!(hv.span.line, 3);
    assert_eq!(
        hv.pieces,
        vec![
            PathSeg::Line {
                from: Pt::new(60.0, 20.0),
                to: Pt::new(200.0, 20.0)
            },
            PathSeg::Line {
                from: Pt::new(200.0, 20.0),
                to: Pt::new(200.0, 100.0)
            },
        ]
    );
    assert!(l.segments[2].pieces.is_empty());
    assert_eq!(l.path_seg, vec![0, 0, 1, 1, 1]);
    assert_eq!(l.path.len(), l.path_seg.len());
    assert_eq!(SegmentKind::CurveVia.name(), "curve-via");
}

#[test]
fn literals_are_recorded_in_their_own_frame() {
    let s = ok(
        "group g at (100,50) size (300,200) {\n  box a at (10,20) size (40,30)\n}\nline l a.right+(5,0) -> h 15 -> (300,95)\n",
    );
    let lits: Vec<(&str, &str, f64)> = s
        .literals
        .iter()
        .map(|l| (l.owner.as_str(), l.what, l.value))
        .collect();
    assert_eq!(
        lits,
        [
            ("g", "x", 100.0),
            ("g", "y", 50.0),
            ("g", "width", 300.0),
            ("g", "height", 200.0),
            ("a", "x", 10.0),
            ("a", "y", 20.0),
            ("a", "width", 40.0),
            ("a", "height", 30.0),
            ("l", "dx", 5.0),
            ("l", "dy", 0.0),
            ("l", "h", 15.0),
            ("l", "x", 300.0),
            ("l", "y", 95.0),
        ]
    );
    assert_eq!(s.literals[4].span.line, 3);
}

#[test]
fn line_endpoints_know_whether_they_are_literal() {
    let s = ok(
        "box a at (0,0) size (40,40)\nline m (100,100) -> a.left\nline n a.top -> y -20\nline o a.bottom -> x a.right\nline p a.right -> hv (80,80)\n",
    );
    let flags: Vec<(bool, bool)> = s
        .lines
        .iter()
        .map(|l| (l.start_literal, l.end_literal))
        .collect();
    assert_eq!(
        flags,
        [(true, false), (false, true), (false, false), (false, true)]
    );
}

#[test]
fn boxes_and_groups_accept_allow_and_reason() {
    let s = ok(
        "box a at (0,0) size (10,10) allow=overlap reason=\"badge\"\ngroup g at (0,0) size (50,50) allow=spill:a {\n}\n",
    );
    let a = s.shapes.iter().find(|x| x.id == "a").unwrap();
    assert_eq!(
        a.allow,
        vec![kashika_model::Allow {
            code: "overlap".into(),
            other: None
        }]
    );
    assert_eq!(a.reason.as_deref(), Some("badge"));
    let g = s.shapes.iter().find(|x| x.id == "g").unwrap();
    assert_eq!(g.allow[0].other.as_deref(), Some("a"));
    assert_eq!(
        errors("box a at (0,0) size (10,10) allow=overlpa\n")[0].code,
        "attr"
    );
}

#[test]
fn backward_placement_chains_obey_the_depth_limit() {
    let chain = |n: usize| {
        let mut body = String::from("box b0 at (0,0) size (1,1)\n");
        for i in 1..=n {
            body.push_str(&format!(
                "box b{i} at b{}.right size (1,1) pin=left\n",
                i - 1
            ));
        }
        body
    };
    let s = ok(&chain(256));
    assert_eq!(rect(&s, "b256").x, 256.0);
    let d = errors(&chain(600));
    assert_eq!(codes_of(&d), vec!["placement-cycle"], "{d:#?}");
    assert_eq!(
        d[0].message,
        "placement chain from `b600` to `b0` is 600 references deep; the limit is 256"
    );
    assert_eq!(d[0].span.line, 602);
}

#[test]
fn long_placement_cycles_name_the_cycle() {
    let n = 300;
    let mut body = String::new();
    for i in 0..n {
        body.push_str(&format!("box b{i} at b{}.right size (1,1)\n", (i + 1) % n));
    }
    let d = errors(&body);
    assert_eq!(codes_of(&d), vec!["placement-cycle"], "{d:#?}");
    assert_eq!(
        d[0].message,
        "placement cycle of 300 shapes: b0 -> b1 -> b2 -> b3 -> … -> b299 -> b0"
    );
    assert_eq!((d[0].span.line, d[0].span.col), (2, 11));
}

#[test]
fn shapes_leading_into_a_cycle_are_not_part_of_it() {
    let d = errors(
        "box p at a.right size (10,10)\nbox a at b.right size (10,10)\nbox b at a.left size (10,10)\n",
    );
    assert_eq!(codes_of(&d), vec!["placement-cycle"], "{d:#?}");
    assert_eq!(d[0].message, "placement cycle: a -> b -> a");
}
