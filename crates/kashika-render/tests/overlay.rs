use kashika_diag::{Diagnostic, Severity, Span};
use kashika_model::compile;
use kashika_render::{render_svg, render_svg_with, Overlay};

const SRC: &str = "diagram \"o\" size (300,200)\n\
box a \"A\" at (10,10) size (40,20) colr=#fff\n\
box b \"B\" at (100,100) size (40,20)\n\
line ab a.right -> hv b.top\n\
line bad a.rigth -> b.left\n";

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

#[test]
fn debug_marks_a_through_warning_at_its_entry_point() {
    let src = "diagram \"t\" size (300,100)\n\
box a at (10,30) size (40,40)\n\
box w at (120,20) size (40,60)\n\
box b at (240,30) size (40,40)\n\
line ab a.right -> b.left\n";
    let (scene, _) = compile(src);
    let d = kashika_check::check(&scene)
        .into_iter()
        .find(|d| d.code == "through")
        .expect("through is reported");
    let s = render_svg_with(&scene, std::slice::from_ref(&d), DEBUG);
    assert!(
        s.contains(r##"<circle cx="120" cy="50" r="6" fill="none" stroke="#ea580c""##),
        "{s}"
    );
}

#[test]
fn debug_text_halo_uses_the_background() {
    let halo = |head: &str, color: &str| {
        let (scene, diags) = compile(&format!("{head}\nbox a \"A\" at (10,10) size (80,40)\n"));
        assert!(diags.is_empty(), "{diags:#?}");
        let s = render_svg_with(&scene, &diags, DEBUG);
        let overlay = &s[s.find("kashika-overlay").unwrap()..];
        let want = format!("stroke=\"{color}\" stroke-width=\"3\"");
        overlay.contains(&want)
            && overlay.matches("stroke-width=\"3\"").count() == overlay.matches(&want).count()
    };
    assert!(halo("diagram \"o\" size (300,200)", "#ffffff"));
    assert!(halo(
        "diagram \"o\" size (300,200) background=#0f172a",
        "#0f172a"
    ));
}
