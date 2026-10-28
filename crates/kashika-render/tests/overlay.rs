use kashika_diag::{Diagnostic, Severity, Span};
use kashika_model::compile;
use kashika_render::{render_svg, render_svg_with, Overlay};

const SRC: &str = "diagram \"o\" size (300,200)\n\
box a \"A\" at (10,10) size (40,20) colr=#fff\n\
box b \"B\" at (100,100) size (40,20)\n\
line ab: a.right -> hv b.top\n\
line bad: a.rigth -> b.left\n";

fn warn_on_line_4() -> Diagnostic {
    let mut d = Diagnostic::error("near", Span::new(4, 1, 4, 5), "test warning");
    d.severity = Severity::Warn;
    d
}

const MARKERS: Overlay = Overlay {
    error_markers: true,
    debug: false,
};
const DEBUG: Overlay = Overlay {
    error_markers: false,
    debug: true,
};

#[test]
fn no_overlay_is_the_plain_svg() {
    let (scene, diags) = compile(SRC);
    assert_eq!(
        render_svg_with(&scene, &diags, Overlay::default()),
        render_svg(&scene)
    );
}

#[test]
fn error_markers_frame_elements_and_list_the_rest() {
    let (scene, diags) = compile(SRC);
    let s = render_svg_with(&scene, &diags, MARKERS);
    assert!(
        s.contains(r##"<rect x="7" y="7" width="46" height="26" fill="none" stroke="#dc2626""##),
        "{s}"
    );
    assert!(s.contains(r#"aria-label="error[attr]""#), "{s}");
    assert!(s.contains(r#"error[unknown-anchor]""#), "{s}");
    assert!(s.contains("<g id=\"kashika-overlay\">"));
    assert!(s.ends_with("</g>\n</svg>\n"));
}

#[test]
fn warnings_are_marked_only_in_debug() {
    let (scene, mut diags) = compile(SRC);
    diags.push(warn_on_line_4());
    assert!(!render_svg_with(&scene, &diags, MARKERS).contains("#ea580c"));
    let s = render_svg_with(&scene, &diags, DEBUG);
    assert!(
        s.contains(r##"stroke="#ea580c" stroke-opacity="0.5""##),
        "{s}"
    );
    assert!(s.contains(r#"aria-label="warn[near]""#));
}

#[test]
fn overlay_is_deterministic() {
    let (scene, diags) = compile(SRC);
    let o = Overlay {
        error_markers: true,
        debug: true,
    };
    assert_eq!(
        render_svg_with(&scene, &diags, o),
        render_svg_with(&scene, &diags, o)
    );
}

#[test]
fn debug_draws_rulers_ids_and_anchor_dots() {
    let (scene, diags) = compile(SRC);
    let s = render_svg_with(&scene, &diags, DEBUG);
    assert!(s.contains("M 100 0 V 10"), "x ruler tick at 100");
    assert!(s.contains("M 0 100 H 10"), "y ruler tick at 100");
    for label in ["100", "a", "b", "ab"] {
        assert!(s.contains(&format!("aria-label=\"{label}\"")), "{label}");
    }
    assert_eq!(
        s.matches(r##"r="1.5" fill="#2563eb""##).count(),
        18,
        "9 anchors x 2 boxes"
    );
}
