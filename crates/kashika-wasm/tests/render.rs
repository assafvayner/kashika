use kashika_wasm::{render, render_json};

const CLEAN: &str = "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40)\n";
const BROKEN: &str =
    "diagram \"t\" size (200,100)\nbox a \"A\" at (20,20) size (80,40) icon=\"missing.svg\"\n";

#[test]
fn clean_source_renders_svg() {
    let r = render(CLEAN, false);
    assert_eq!((r.errors, r.warnings), (0, 0));
    assert!(r.svg.unwrap().starts_with("<svg"));
}

#[test]
fn render_carries_element_boxes() {
    let r = render(CLEAN, false);
    assert_eq!(r.elements[0].id, "a");
    assert_eq!(r.elements[0].bbox, [[20.0, 20.0], [100.0, 60.0]]);
    let json: serde_json::Value = serde_json::from_str(&render_json(CLEAN, false)).unwrap();
    assert!(json["elements"].is_array());
}

#[test]
fn file_references_are_not_found() {
    let r = render(BROKEN, false);
    assert_eq!(r.errors, 1);
    assert_eq!(r.diagnostics[0].code, "file");
    assert!(r.svg.is_none());
}

#[test]
fn force_renders_despite_errors() {
    let r = render(BROKEN, true);
    assert_eq!(r.errors, 1);
    let svg = r.svg.unwrap();
    let at = svg
        .find("<g id=\"kashika-overlay\">\n")
        .expect("overlay group");
    assert!(svg[at..].contains("aria-label=\"error[file]\""), "{svg}");
    assert!(svg.ends_with("</g>\n</svg>\n"));
    assert!(!render(CLEAN, true).svg.unwrap().contains("kashika-overlay"));
}

#[test]
fn library_has_no_svg() {
    let r = render("style s { fill=\"#ffffff\" }\n", true);
    assert!(r.svg.is_none());
}

#[test]
fn json_has_the_report_fields() {
    let v: serde_json::Value = serde_json::from_str(&render_json(BROKEN, false)).unwrap();
    assert!(v["svg"].is_null());
    assert_eq!(v["errors"], 1);
    assert_eq!(v["warnings"], 0);
    assert_eq!(v["diagnostics"][0]["code"], "file");
    assert_eq!(v["diagnostics"][0]["span"]["line"], 2);
}

#[test]
fn matches_the_cli_render_of_the_upload_example() {
    let src = include_str!("../../../examples/upload/upload.kash");
    let svg = render(src, false).svg.unwrap();
    assert_eq!(svg, include_str!("../../../examples/upload/upload.svg"));
}

#[test]
fn diagnostics_are_sorted_when_there_is_no_scene() {
    let r = render("box a \"A\" at (0,0) size (10,10)\nbox b\n", false);
    let lines: Vec<_> = r.diagnostics.iter().map(|d| d.span.line).collect();
    assert!(lines.windows(2).all(|w| w[0] <= w[1]), "{lines:?}");
}
