mod common;

use common::*;

#[test]
fn missing_glyph_flags_each_text_kind() {
    let d = lint(
        "missing-glyph",
        "box a \"日本\" at (20,20) size (200,80) sublabel=\"ok\"\n\
         box b \"fine é\" at (200,20) size (120,40)\n\
         text t \"x 日 **日** y\" at (20,120)\n\
         line l a.right -> b.left label=\"→ 日\"\n",
    );
    assert_eq!(d.len(), 3, "{d:#?}");
    assert_eq!(d[0].span.line, 2);
    assert_eq!(
        d[0].facts["codepoints"],
        serde_json::json!(["U+65E5", "U+672C"])
    );
    assert_eq!(d[1].elements[0].kind, "text");
    assert_eq!(d[1].facts["codepoints"], serde_json::json!(["U+65E5"]));
    assert_eq!(d[2].elements[0].kind, "line");
}

#[test]
fn missing_glyph_is_quiet_for_covered_text_and_tabs() {
    assert!(
        lint(
            "missing-glyph",
            "box a \"Café Ünï Ωmega Привет\" at (20,20) size (240,60)\n\
             text t \"a\tb\" at (20,120)\n\
             text u \"zero\u{200B}width\u{200D}join\u{FE0F}soft\u{00AD}hyphen\u{FEFF}\" at (20,160)\n",
        )
        .is_empty()
    );
}
