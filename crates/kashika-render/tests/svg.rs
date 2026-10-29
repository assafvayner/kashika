use kashika_model::compile;
use kashika_render::render_svg;

const BASIC: &str = "diagram \"Basic\" size (480,240) kashika=\"0.1.0\"\n\
box api \"API\" at (40,40) size (120,60) style=service\n\
box db \"Postgres\" at api.right+(160,0) size (120,60) pin=left style=datastore\n\
line api_db api.right -> db.left end=arrow label=\"SQL\"\n";

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
    let s = svg(
        "diagram \"g\" size (200,200)\ngroup g \"VPC\" at (10,10) size (180,180) {\n  box a at (10,30) size (50,30)\n}\n",
    );
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
fn paint_order_is_groups_boxes_lines_then_overlays_in_file_order() {
    let s = svg("diagram \"p\" size (300,300)\n\
                 text \"T\" at (5,5)\n\
                 line l (0,150) -> h 300 label=\"L\"\n\
                 box b at (100,100) size (50,50)\n\
                 group outer at (0,0) size (300,300) {\n  group inner at (20,20) size (100,100) {\n  }\n}\n");
    let pos = |needle: &str| s.find(needle).unwrap_or_else(|| panic!("{needle}"));
    let outer = pos(r#"<rect x="0" y="0" width="300""#);
    let inner = pos(r#"<rect x="20" y="20""#);
    let bx = pos(r#"<rect x="100" y="100""#);
    let line = pos(r#"d="M 0 150 L 300 150""#);
    let label = pos(r#"aria-label="L""#);
    let text = pos(r#"aria-label="T""#);
    assert!(outer < inner && inner < bx && bx < line && line < text && text < label);
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
    let s = svg(
        "diagram \"c\" size (200,200)\nline l (0,100) -> curve left 20 (100,100) end=arrow start=diamond\n",
    );
    let d = path_d(&s);
    assert!(d.starts_with("M ") && !d.starts_with("M 0 100"), "{d}");
    assert!(!d.ends_with(" 100 100"), "{d}");
}

#[test]
fn markers_at_both_ends_of_a_short_line_share_it() {
    let s = svg("diagram \"c\" size (200,200)\nline l (0,50) -> h 15 start=arrow end=arrow\n");
    assert_eq!(path_d(&s), "M 7.5 50 L 7.5 50");
}

#[test]
fn line_labels_get_a_mask_and_texts_do_not() {
    let s = svg(
        "diagram \"h\" size (200,100)\nline l (0,50) -> h 200 label=\"L\"\ntext \"T\" at (100,90)\n",
    );
    let rect = r##"fill="#000"/></mask>"##;
    let mask = s.find(rect).expect("label rect in the mask");
    let id = mask_ids(&s)[0].clone();
    let group = s.find(&format!(r#"<g mask="url(#{id})">"#)).unwrap();
    let path = s.find("<path d=\"M 0 50").unwrap();
    let close = s.find("</g>").unwrap();
    assert!(mask < group && group < path && path < close, "{s}");
    assert!(close < s.find(r#"aria-label="L""#).unwrap());
    assert!(!s.contains("stroke-width=\"4\""));
    let text = &s[s.find(r#"aria-label="T""#).unwrap()..];
    assert!(
        text.lines()
            .next()
            .unwrap()
            .ends_with(r##"fill="#0f172a"/>"##)
    );
    assert_eq!(s.matches("<mask ").count(), 1, "only the line gets a mask");
    assert_eq!(
        s.matches("<rect ").count(),
        2,
        "the mask's base and one label"
    );
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

#[test]
fn nested_groups_paint_before_later_sibling_groups() {
    let s = svg(
        "diagram \"g\" size (400,200)\ngroup a \"A\" at (10,10) size (180,180) {\n  group a2 \"A2\" at (10,30) size (100,100) {\n  }\n}\ngroup b \"B\" at (210,10) size (180,180) {\n}\n",
    );
    let pos = |label: &str| s.find(&format!("aria-label=\"{label}\"")).unwrap();
    assert!(pos("A") < pos("A2") && pos("A2") < pos("B"), "{s}");
}

#[test]
fn line_labels_and_texts_paint_in_file_order() {
    let s = svg(
        "diagram \"t\" size (200,100)\ntext \"T\" at (10,10)\nline l (0,50) -> h 100 label=\"L\"\n",
    );
    assert!(
        s.find("aria-label=\"T\"").unwrap() < s.find("aria-label=\"L\"").unwrap(),
        "{s}"
    );
}

#[test]
fn canvas_is_transparent_unless_background_is_set() {
    let canvas = r#"<rect width="100%" height="100%""#;
    let s = svg("diagram \"t\" size (100,100)\nbox a at (0,0) size (50,50)\n");
    assert!(!s.contains(canvas), "{s}");
    let s = svg("diagram \"t\" size (100,100) background=none\nbox a at (0,0) size (50,50)\n");
    assert!(!s.contains(canvas), "{s}");
    let s = svg("diagram \"t\" size (100,100) background=#0f172a\nbox a at (0,0) size (50,50)\n");
    let bg = s
        .find(r##"<rect width="100%" height="100%" fill="#0f172a"/>"##)
        .expect("canvas rect");
    let first_box = s.find(r#"<rect x="0" y="0""#).expect("box");
    assert!(bg < first_box);
}

#[test]
fn line_labels_mask_their_own_line_without_painting() {
    let s = svg("diagram \"t\" size (300,100)\nline l (20,50) -> (280,50) label=\"open\"\n");
    assert!(!s.contains("#ffffff"), "{s}");
    assert!(s.contains("<mask "), "{s}");
    assert!(s.contains(r#"mask="url(#"#), "{s}");
    assert_eq!(
        s,
        svg("diagram \"t\" size (300,100)\nline l (20,50) -> (280,50) label=\"open\"\n")
    );
}

#[test]
fn named_colors_render_as_hex() {
    let s = svg(
        "diagram \"t\" size (100,100) background=ivory\nbox a \"x\" at (10,10) size (40,40) fill=Tomato color=white\n",
    );
    for hex in ["#fffff0", "#ff6347", "#ffffff"] {
        assert!(s.contains(hex), "{hex} missing in {s}");
    }
    for name in ["ivory", "tomato", "Tomato", "white"] {
        assert!(!s.contains(name), "{name} leaked into {s}");
    }
}

fn mask_ids(svg: &str) -> Vec<String> {
    svg.match_indices("<mask id=\"")
        .map(|(i, m)| {
            let rest = &svg[i + m.len()..];
            rest[..rest.find('"').unwrap()].to_string()
        })
        .collect()
}

#[test]
fn mask_ids_are_unique_per_diagram_and_stable() {
    let a = "diagram \"a\" size (200,100)\nline l (0,50) -> h 200 label=\"L\"\nline m (0,80) -> h 200 label=\"M\"\n";
    let b = "diagram \"b\" size (200,100)\nline l (0,50) -> h 200 label=\"L\"\n";
    let (ia, ib) = (mask_ids(&svg(a)), mask_ids(&svg(b)));
    assert_eq!(ia.len(), 2);
    for id in ia.iter().chain(&ib) {
        let hash = id
            .strip_prefix("kashika-")
            .and_then(|r| r.split_once("-mask-"))
            .map(|(h, _)| h)
            .unwrap_or_else(|| panic!("{id}"));
        assert!(
            hash.len() == 8 && hash.bytes().all(|c| c.is_ascii_hexdigit()),
            "{id}"
        );
    }
    assert_ne!(ia[0], ia[1]);
    assert!(ib.iter().all(|id| !ia.contains(id)), "{ia:?} {ib:?}");
    assert_eq!(ia, mask_ids(&svg(a)));
}

#[test]
fn drops_characters_invalid_in_xml() {
    let (mut scene, diags) =
        compile("diagram \"e\" size (100,100)\nbox a \"ab\" at (0,0) size (90,40)\n");
    assert!(diags.is_empty(), "{diags:#?}");
    scene.shapes[0].text[0].lines[0].runs[0].text = "a\u{1}\u{b}\u{fffe}\u{ffff}b\tc".to_string();
    let s = render_svg(&scene);
    assert!(s.contains("aria-label=\"ab\tc\""), "{s}");
}

#[test]
fn repeated_icons_are_embedded_once() {
    let s = svg(
        "diagram \"r\" size (400,100)\nbox a at (0,0) size (60,60) icon=\"server\"\nbox b at (100,0) size (60,60) icon=\"server\"\nbox c at (200,0) size (60,60) icon=\"lock\"\n",
    );
    assert_eq!(s.matches("data:image/svg+xml;base64,").count(), 2, "{s}");
    assert_eq!(s.matches("<use href=\"#kashika-").count(), 3, "{s}");
}
