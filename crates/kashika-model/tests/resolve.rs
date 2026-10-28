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
