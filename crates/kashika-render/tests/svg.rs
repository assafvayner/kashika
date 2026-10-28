use kashika_model::compile;
use kashika_render::render_svg;

const BASIC: &str = "diagram \"Basic\" size (480,240) kashika=1\n\
box api \"API\" at (40,40) size (120,60) style=service\n\
box db \"Postgres\" at api.right+(160,0) size (120,60) pin=left style=datastore\n\
line api_db: api.right -> db.left end=arrow label=\"SQL\"\n";

fn svg(src: &str) -> String {
    let (scene, diags) = compile(src);
    assert!(diags.is_empty(), "{diags:#?}");
    render_svg(&scene)
}

#[test]
fn renders_boxes_lines_markers_and_labels() {
    let s = svg(BASIC);
    assert!(s.starts_with(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="480" height="240" viewBox="0 0 480 240">"#
    ));
    assert!(s.contains(
        r##"<rect x="40" y="40" width="120" height="60" rx="0" fill="#e8f0fe" stroke="#1a73e8" stroke-width="1.5"/>"##
    ));
    assert!(
        s.contains(r#"d="M 160 70 L 310 70""#),
        "arrow end should trim the stroke"
    );
    assert!(s.contains(r#"<polygon points="320,70 310,74 310,66""#));
    assert!(s.contains(r#"<path aria-label="SQL" d="M"#));
    assert!(s.contains(r#"<path aria-label="Postgres" d="M"#));
    assert!(!s.contains("<text"), "text is drawn as glyph outlines");
}

#[test]
fn groups_are_dashed_and_drawn_first() {
    let s = svg("diagram \"g\" size (200,200)\ngroup g \"VPC\" at (10,10) size (180,180) {\n  box a at (10,30) size (50,30)\n}\n");
    let group = s.find(r#"stroke-dasharray="6 4""#).expect("dashed group");
    let child = s.find(r#"<rect x="20" y="40""#).expect("child box");
    assert!(group < child);
    assert!(s.contains(r#"aria-label="VPC""#));
}

#[test]
fn escapes_text() {
    let s = svg("diagram \"e\" size (100,100)\nbox a \"a < b & c\" at (0,0) size (90,40)\n");
    assert!(s.contains(r#"aria-label="a &lt; b &amp; c""#));
}

#[test]
fn multi_line_labels_stack() {
    let s = svg("diagram \"m\" size (100,100)\nbox a \"one\\ntwo\" at (0,0) size (100,100)\n");
    let one = s.find(r#"aria-label="one""#).expect("first line");
    let two = s.find(r#"aria-label="two""#).expect("second line");
    assert!(one < two);
}

#[test]
fn snapshot_basic() {
    insta::assert_snapshot!("basic", svg(BASIC));
}

#[test]
fn paint_order_is_groups_boxes_lines_then_labels_and_texts() {
    let s = svg("diagram \"p\" size (300,300)\n\
                 text \"T\" at (5,5)\n\
                 line l: (0,150) -> h 300 label=\"L\"\n\
                 box b at (100,100) size (50,50)\n\
                 group outer at (0,0) size (300,300) {\n  group inner at (20,20) size (100,100) {\n  }\n}\n");
    let pos = |needle: &str| s.find(needle).unwrap_or_else(|| panic!("{needle}"));
    let outer = pos(r#"<rect x="0" y="0" width="300""#);
    let inner = pos(r#"<rect x="20" y="20""#);
    let bx = pos(r#"<rect x="100" y="100""#);
    let line = pos(r#"d="M 0 150 L 300 150""#);
    let label = pos(r#"aria-label="L""#);
    let text = pos(r#"aria-label="T""#);
    assert!(outer < inner && inner < bx && bx < line && line < label && label < text);
}

#[test]
fn junction_dot_is_six_pixels() {
    let s = svg("diagram \"j\" size (100,100)\njunction a at (10,20) dot\njunction b at (30,40)\n");
    assert!(s.contains(r#"<circle cx="10" cy="20" r="3""#));
    assert!(!s.contains(r#"cx="30""#));
}

fn path_d(s: &str) -> &str {
    let start = s.find("<path d=\"").unwrap() + 9;
    &s[start..start + s[start..].find('"').unwrap()]
}

#[test]
fn markers_trim_a_final_curve() {
    let s = svg("diagram \"c\" size (200,200)\nline l: (0,100) -> curve left:20 (100,100) end=arrow start=diamond\n");
    let d = path_d(&s);
    assert!(d.starts_with("M ") && !d.starts_with("M 0 100"), "{d}");
    assert!(!d.ends_with(" 100 100"), "{d}");
}

#[test]
fn markers_at_both_ends_of_a_short_line_share_it() {
    let s = svg("diagram \"c\" size (200,200)\nline l: (0,50) -> h 15 start=arrow end=arrow\n");
    assert_eq!(path_d(&s), "M 7.5 50 L 7.5 50");
}

#[test]
fn line_labels_get_a_halo_and_texts_do_not() {
    let s = svg("diagram \"h\" size (200,100)\nline l: (0,50) -> h 200 label=\"L\"\ntext \"T\" at (100,90)\n");
    let label = &s[s.find(r#"aria-label="L""#).unwrap()..];
    assert!(label.lines().next().unwrap().ends_with(
        r##"fill="#0f172a" stroke="#ffffff" stroke-width="4" stroke-linejoin="round" paint-order="stroke"/>"##
    ));
    let text = &s[s.find(r#"aria-label="T""#).unwrap()..];
    assert!(text
        .lines()
        .next()
        .unwrap()
        .ends_with(r##"fill="#0f172a"/>"##));
}

#[test]
fn markup_switches_fonts_in_one_path() {
    let plain = svg("diagram \"m\" size (200,100)\ntext \"ab\" at (100,50)\n");
    let bold = svg("diagram \"m\" size (200,100)\ntext \"**ab**\" at (100,50)\n");
    assert!(bold.contains(r#"aria-label="ab""#));
    assert_ne!(plain, bold);
}

#[test]
fn spaces_only_lines_draw_nothing() {
    let s = svg("diagram \"s\" size (100,100)\ntext \"  \" at (50,50)\n");
    assert!(!s.contains("aria-label"));
}
