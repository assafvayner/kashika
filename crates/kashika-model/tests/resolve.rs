use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, Rect};
use kashika_model::{compile, Scene, StrokeStyle};

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
    let s = ok("box a at (200,0) size (100,40)\ngroup g at (0,100) size (400,300) {\n  junction j at (a.left.x, 50)\n}\n");
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

#[test]
fn version_mismatch() {
    let (_, d) = compile("diagram \"t\" size (1,1) kashika=2\n");
    assert_eq!(d[0].code, "version");
}

#[test]
fn allow_validates_codes_and_ids() {
    let d = errors("line l1: (0,0) -> h 5 allow=crosss\nline l2: (0,0) -> v 5 allow=cross:nope\n");
    assert!(d
        .iter()
        .any(|d| d.code == "attr" && d.help == vec!["did you mean `cross`?".to_string()]));
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
    let s = ok("box r at (0,0) size (100,60) shape=rounded\nline l: r.top@0 -> (0,-50)\n");
    assert_eq!(pts(&s, "l")[0], Pt::new(0.0, 8.0));
}

#[test]
fn every_segment_kind_resolves() {
    let s = ok("box a at (0,0) size (100,60)\nbox b at (300,200) size (100,60)\n\
                line l: a.right -> h 50 -> y b.top -> x 400 -> v 10 -> hv (500,300) -> vh (520,320)\n");
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
    let s = ok("line l: (0,0) -> hv (100,0)\n");
    assert_eq!(s.lines[0].path.len(), 1);
}

#[test]
fn numeric_axis_targets_use_the_group_frame() {
    let s = ok("group g at (100,100) size (300,300) {\n  line l: (0,0) -> x 50 -> y 20\n}\n");
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
    let s = ok("line l: (0,100) -> curve left:20 (100,100)\n");
    let PathSeg::Quad { ctrl, .. } = s.lines[0].path[0] else {
        panic!("expected quad")
    };
    assert_eq!(ctrl, Pt::new(50.0, 60.0));
    assert_eq!(s.lines[0].path[0].eval(0.5), Pt::new(50.0, 80.0));
}

#[test]
fn curve_right_bulges_right_when_travelling_up() {
    let s = ok("line l: (0,100) -> curve right:10 (0,0)\n");
    let PathSeg::Quad { ctrl, .. } = s.lines[0].path[0] else {
        panic!("expected quad")
    };
    assert_eq!(ctrl, Pt::new(20.0, 50.0));
}

#[test]
fn curve_via_makes_a_cubic() {
    let s = ok("line l: (0,0) -> curve via (0,50) (100,50) (100,0)\n");
    assert!(matches!(s.lines[0].path[0], PathSeg::Cubic { .. }));
}

#[test]
fn radius_rounds_polyline_corners() {
    let s = ok("line l: (0,0) -> h 100 -> v 100 radius=10\n");
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
    let d = errors("line l: (0,0) -> (10,10) ortho\n");
    assert_eq!(d[0].code, "not-ortho");
}

#[test]
fn degenerate_curve() {
    let d = errors("line l: (0,0) -> curve left:10 (0,0)\n");
    assert_eq!(d[0].code, "degenerate-curve");
}

#[test]
fn zero_length_line() {
    let d = errors("line l: (0,0) -> h 0\n");
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
        codes("line l: (0,0) -> nope.left -> curve left:10 (0,0)\n"),
        vec!["unknown-id"]
    );
}

#[test]
fn non_finite_values_are_rejected() {
    let big = "9".repeat(400);
    for body in [
        format!("box a at ({big},0) size (1,1)\n"),
        format!("box a at (0,0) size ({big},1)\n"),
        format!("line l: (0,0) -> h {big}\n"),
        format!("line l: (0,0) -> v {big}\n"),
        format!("line l: (0,0) -> curve left:{big} (10,0)\n"),
        format!("line l: (0,0) -> curve via ({big},0) (1,1) (2,2)\n"),
    ] {
        let d = errors(&body);
        assert!(
            d.iter().any(
                |d| d.code == "degenerate-curve" && d.message == "value is not a finite number"
            ),
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
    let d = errors("line l: (0,0) -> h 10 label=\"a\" label@0.5=\"b\"\n");
    assert_eq!(
        d.iter()
            .filter(|d| d.code == "attr" && d.message.starts_with("duplicate label at t="))
            .count(),
        1
    );
    let (s, _) = compile(&format!(
        "{HEAD}line l: (0,0) -> h 10 label@0.3=\"a\" label@0.3=\"b\"\n"
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
    let d = errors("line l: (0,0) -> curve left:-10 (10,0)\n");
    assert_eq!(d[0].code, "degenerate-curve");
    assert_eq!(
        d[0].message,
        "curve amount must not be negative; use `left`/`right` to choose the side"
    );
}

#[test]
fn unsupported_features_fail_clearly() {
    let d = errors("box a at (0,0) size (10,10) sublabel=\"x\"\n");
    assert_eq!(d[0].code, "unsupported");
    assert_eq!(
        d[0].message,
        "`sublabel` is not supported in this version yet"
    );
    let d = errors("box a at (0,0) size (10,10) shape=cylinder\n");
    assert_eq!(d[0].code, "unsupported");
    assert_eq!(
        d[0].message,
        "`cylinder` is not supported in this version yet"
    );
    let d = errors("group g at (0,0) size (10,10) icon=\"x\" {\n}\n");
    assert_eq!(d[0].code, "unsupported");
    let d = errors("box a at (0,0) size (10,10) fil=#fff\n");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn duplicate_label_attribute_reports_once() {
    let d = errors("line l: (0,0) -> h 5 label@0.3=\"a\" label@0.3=\"b\"\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "attr");
}

#[test]
fn duplicate_unsupported_attribute_reports_once() {
    let d = errors("box a at (0,0) size (1,1) sublabel=\"x\" sublabel=\"y\"\n");
    let codes: Vec<_> = d.iter().map(|x| x.code).collect();
    assert_eq!(codes.iter().filter(|c| **c == "attr").count(), 1, "{d:#?}");
    assert_eq!(
        codes.iter().filter(|c| **c == "unsupported").count(),
        1,
        "{d:#?}"
    );
    assert_eq!(d.len(), 2, "{d:#?}");
}

#[test]
fn zero_curve_amount_is_allowed() {
    ok("line l: (0,0) -> curve left:0 (10,0)\n");
}

#[test]
fn negative_curve_message() {
    let d = errors("line l: (0,0) -> curve left:-1 (10,0)\n");
    assert!(
        d.iter().any(|x| x.message.contains("must not be negative")),
        "{d:#?}"
    );
}

#[test]
fn children_of_duplicate_group_are_not_reference_targets() {
    let d = errors(
        "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  box b at (0,0) size (10,10)\n}\nline l: (0,0) -> b.left\n",
    );
    assert!(d.iter().any(|x| x.code == "duplicate-id"), "{d:#?}");
    assert!(d.iter().any(|x| x.code == "unknown-id"), "{d:#?}");
}

#[test]
fn grandchildren_of_duplicate_group_are_not_reference_targets() {
    let d = errors(
        "group g at (0,0) size (100,100) {\n}\ngroup g at (200,0) size (100,100) {\n  group h at (0,0) size (50,50) {\n    box b at (0,0) size (10,10)\n  }\n}\nline l: (0,0) -> b.left\n",
    );
    assert!(
        d.iter()
            .any(|d| d.code == "unknown-id" && d.message.contains("`b`")),
        "{d:#?}"
    );
}
