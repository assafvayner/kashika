//! Checks every rendered SVG must pass, whatever the case draws.

use std::collections::HashSet;

use base64::Engine;
use kashika_model::Scene;
use kashika_text::PlacedLine;
use roxmltree::Document;

const XLINK: &str = "http://www.w3.org/1999/xlink";

/// Returns one message per violated invariant of `svg`, rendered from `scene`; empty when
/// the SVG is sound.
pub fn check(svg: &str, scene: &Scene) -> Vec<String> {
    let mut problems = Vec::new();
    let doc = match Document::parse(svg) {
        Ok(d) => d,
        Err(e) => return vec![format!("not well-formed XML: {e}")],
    };
    if let Err(e) = usvg::Tree::from_str(svg, &usvg::Options::default()) {
        problems.push(format!("usvg cannot parse the output: {e}"));
    }
    if doc.descendants().any(|n| n.has_tag_name("text")) {
        problems.push("output contains a <text> element; text must be glyph outlines".into());
    }
    let canvas_fills: Vec<Option<&str>> = doc
        .descendants()
        .filter(|n| n.has_tag_name("rect"))
        .filter(|n| n.attribute("width") == Some("100%") || n.attribute("height") == Some("100%"))
        .map(|n| n.attribute("fill"))
        .collect();
    match (&scene.background, canvas_fills.as_slice()) {
        (None, []) => {}
        (None, _) => problems.push("a 100% rect paints the canvas without `background=`".into()),
        (Some(bg), [fill]) if *fill == Some(bg.as_str()) => {}
        (Some(bg), _) => problems.push(format!(
            "`background={bg}` should paint exactly one 100% rect with that fill, found {canvas_fills:?}"
        )),
    }
    for img in doc.descendants().filter(|n| n.has_tag_name("image")) {
        let Some(href) = img.attribute("href").or(img.attribute((XLINK, "href"))) else {
            problems.push("<image> without href".into());
            continue;
        };
        if let Some(b64) = href.strip_prefix("data:image/svg+xml;base64,") {
            match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(bytes) => embedded_svg(&String::from_utf8_lossy(&bytes), &mut problems),
                Err(e) => problems.push(format!("embedded SVG is not base64: {e}")),
            }
        } else if !href.starts_with("data:image/") {
            problems.push(format!("<image> href is not a data URI: {href}"));
        }
    }
    let labels: HashSet<&str> = doc
        .descendants()
        .filter_map(|n| n.attribute("aria-label"))
        .collect();
    for line in scene_lines(scene) {
        let plain: String = line.runs.iter().map(|r| r.text.as_str()).collect();
        if !plain.trim().is_empty() && !labels.contains(plain.as_str()) {
            problems.push(format!(
                "text line {plain:?} has no aria-label in the output"
            ));
        }
    }
    problems
}

fn scene_lines(scene: &Scene) -> impl Iterator<Item = &PlacedLine> {
    let shapes = scene
        .shapes
        .iter()
        .flat_map(|s| s.text.iter().flat_map(|t| t.lines.iter()));
    let texts = scene.texts.iter().flat_map(|t| t.lines.iter());
    let labels = scene
        .lines
        .iter()
        .flat_map(|l| l.labels.iter().flat_map(|lb| lb.lines.iter()));
    shapes.chain(texts).chain(labels)
}

/// Embedded SVG is the usvg re-serialization: well-formed, with no scripts, foreign objects
/// or event handlers.
fn embedded_svg(inner: &str, problems: &mut Vec<String>) {
    let doc = match Document::parse(inner) {
        Ok(d) => d,
        Err(e) => {
            problems.push(format!("embedded SVG is not well-formed: {e}"));
            return;
        }
    };
    for n in doc.descendants().filter(|n| n.is_element()) {
        let tag = n.tag_name().name();
        if tag.eq_ignore_ascii_case("script") || tag.eq_ignore_ascii_case("foreignObject") {
            problems.push(format!("embedded SVG contains <{tag}>"));
        }
        for a in n.attributes() {
            if a.name().to_ascii_lowercase().starts_with("on") {
                problems.push(format!(
                    "embedded SVG has event handler `{}` on <{tag}>",
                    a.name()
                ));
            }
            let external = (a.name() == "href" || a.namespace() == Some(XLINK))
                && !a.value().starts_with('#')
                && !a.value().starts_with("data:");
            if external {
                problems.push(format!(
                    "embedded SVG references `{}` on <{tag}>",
                    a.value()
                ));
            }
        }
    }
}
