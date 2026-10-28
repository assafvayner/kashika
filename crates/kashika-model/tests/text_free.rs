use kashika_diag::Diagnostic;
use kashika_geom::Pt;
use kashika_model::{compile, Font, Scene};

const HEAD: &str = "diagram \"t\" size (800,600)\n";

fn ok(body: &str) -> Scene {
    let (scene, diags) = compile(&format!("{HEAD}{body}"));
    assert!(diags.is_empty(), "{diags:#?}");
    scene
}

fn errors(body: &str) -> Vec<Diagnostic> {
    compile(&format!("{HEAD}{body}")).1
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn free_text_is_measured_around_its_anchor() {
    let s = ok("text t \"hi\" at (50,40) align=left\ntext u \"note\" at (0,0) muted\n");
    let t = &s.texts[0];
    assert_eq!((t.rect.x, t.rect.y, t.rect.h), (50.0, 31.25, 17.5));
    assert!(close(t.rect.w, 11.6689));
    assert_eq!(t.lines.len(), 1);
    assert_eq!(s.texts[1].color, "#64748b");
}

#[test]
fn color_wins_over_muted() {
    let s = ok("text \"x\" at (0,0) muted color=#ff0000\n");
    assert_eq!(s.texts[0].color, "#ff0000");
}

#[test]
fn free_text_markup_picks_fonts() {
    let s = ok("text \"a `b`\" at (100,100)\n");
    let fonts: Vec<Font> = s.texts[0].lines[0].runs.iter().map(|r| r.font).collect();
    assert_eq!(fonts, vec![Font::Regular, Font::Mono]);
}

#[test]
fn line_labels_are_placed_on_the_path() {
    let s = ok("line l (0,100) -> h 200 label=\"SQL\" label@0.25=\"a\"\n");
    let l = &s.lines[0];
    assert_eq!(l.labels[0].at, Pt::new(100.0, 100.0));
    assert!(close(l.labels[0].rect.center().y, 100.0));
    assert!(close(l.labels[0].rect.w, 23.6602));
    assert_eq!(l.labels[1].at, Pt::new(50.0, 100.0));
    assert_eq!(l.labels[0].span.line, 2);
}

#[test]
fn unclosed_markup_in_free_text_and_labels_is_a_parse_error() {
    let d = errors("text \"`x\" at (0,0)\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    assert_eq!(d[0].message, "unclosed ``` in text");
    let d = errors("line l (0,0) -> h 50 label=\"**x\"\n");
    assert_eq!(d.len(), 1, "{d:#?}");
    assert_eq!(d[0].code, "parse");
    assert_eq!(d[0].message, "unclosed `**` in text");
}

#[test]
fn escapes_are_literal_and_raw_strings_are_kept() {
    let s = ok("text \"\\*\\*x\\*\\* \\\\ y\" at (0,0)\nline l (0,0) -> h 5 label=\"\\`\"\n");
    assert_eq!(s.texts[0].content, "\\*\\*x\\*\\* \\\\ y");
    let runs: Vec<_> = s.texts[0].lines[0].runs.iter().collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "**x** \\ y");
    assert_eq!(s.lines[0].labels[0].lines[0].runs[0].text, "`");
}

#[test]
fn escaped_backslash_does_not_escape_the_next_marker() {
    let s = ok("text \"\\\\**b**\" at (0,0)\n");
    let runs = &s.texts[0].lines[0].runs;
    assert_eq!((runs[0].text.as_str(), runs[0].font), ("\\", Font::Regular));
    assert_eq!((runs[1].text.as_str(), runs[1].font), ("b", Font::Bold));
}
