#[path = "golden/svg_invariants.rs"]
mod svg_invariants;

use base64::Engine;
use kashika_model::Scene;

const EMPTY: &str = "diagram \"t\" size (100,100)\n";

fn scene(src: &str) -> Scene {
    let (scene, diags) = kashika_model::compile(src);
    assert!(diags.is_empty(), "{diags:?}");
    scene
}

fn svg(body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\" viewBox=\"0 0 100 100\">\n{body}</svg>\n"
    )
}

fn problems(body: &str, src: &str) -> Vec<String> {
    scene(src);
    svg_invariants::check_source(&svg(body), src, &kashika_model::Options::default())
}

fn assert_fires(problems: &[String], needle: &str) {
    assert!(
        problems.iter().any(|p| p.contains(needle)),
        "expected a problem containing {needle:?}, got {problems:?}"
    );
}

fn embedded(inner: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(inner);
    format!("<image width=\"10\" height=\"10\" href=\"data:image/svg+xml;base64,{b64}\"/>\n")
}

const INNER_OK: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><rect width=\"10\" height=\"10\"/></svg>";

#[test]
fn sound_svg_passes() {
    assert_eq!(problems("", EMPTY), Vec::<String>::new());
    assert_eq!(problems(&embedded(INNER_OK), EMPTY), Vec::<String>::new());
}

#[test]
fn malformed_xml() {
    let p = svg_invariants::check("<svg", &scene(EMPTY));
    assert_fires(&p, "not well-formed XML");
}

#[test]
fn text_element() {
    assert_fires(&problems("<text>hi</text>\n", EMPTY), "<text> element");
}

#[test]
fn canvas_rect_without_background() {
    assert_fires(
        &problems(
            "<rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>\n",
            EMPTY,
        ),
        "without `background=`",
    );
}

#[test]
fn background_without_canvas_rect() {
    let src = "diagram \"t\" size (100,100) background=#ffffff\n";
    assert_fires(&problems("", src), "should paint exactly one 100% rect");
}

#[test]
fn background_with_wrong_fill() {
    let src = "diagram \"t\" size (100,100) background=#ffffff\n";
    let s = scene(src);
    let bg = s.background.clone().unwrap();
    let ok = svg(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>\n"
    ));
    assert_eq!(svg_invariants::check(&ok, &s), Vec::<String>::new());
    assert_fires(
        &problems("<rect width=\"100%\" height=\"100%\" fill=\"red\"/>\n", src),
        "should paint exactly one 100% rect",
    );
}

#[test]
fn text_line_without_aria_label() {
    let src = "diagram \"t\" size (100,100)\ntext \"hi\" at (10,20)\n";
    assert_fires(&problems("", src), "text line \"hi\" has no aria-label");
    assert_eq!(
        problems("<path aria-label=\"hi\" d=\"M0 0h1v1z\"/>\n", src),
        Vec::<String>::new()
    );
}

#[test]
fn repeated_text_line_needs_one_label_each() {
    let src = "diagram \"t\" size (100,100)\ntext \"hi\" at (10,20)\ntext \"hi\" at (10,60)\n";
    let one = "<path aria-label=\"hi\" d=\"M0 0h1v1z\"/>\n";
    assert_fires(&problems(one, src), "text line \"hi\" has no aria-label");
    assert_eq!(problems(&one.repeat(2), src), Vec::<String>::new());
}

#[test]
fn embedded_svg_with_script() {
    let inner = "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
    assert_fires(
        &problems(&embedded(inner), EMPTY),
        "embedded SVG contains <script>",
    );
}

#[test]
fn embedded_svg_with_event_handler() {
    let inner = "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"1\" height=\"1\" onclick=\"x()\"/></svg>";
    assert_fires(
        &problems(&embedded(inner), EMPTY),
        "event handler `onclick`",
    );
}

#[test]
fn embedded_svg_with_external_href() {
    let inner = "<svg xmlns=\"http://www.w3.org/2000/svg\"><image width=\"1\" height=\"1\" href=\"https://example.com/a.png\"/></svg>";
    assert_fires(
        &problems(&embedded(inner), EMPTY),
        "embedded SVG references `https://example.com/a.png`",
    );
}

#[test]
fn image_href_not_a_data_uri() {
    assert_fires(
        &problems(
            "<image width=\"10\" height=\"10\" href=\"pic.png\"/>\n",
            EMPTY,
        ),
        "<image> href is not a data URI: pic.png",
    );
}
