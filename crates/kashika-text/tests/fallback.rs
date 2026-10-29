use kashika_text::{Run, color_layers, measure, only_emoji, outline, parse_markup};

fn runs(s: &str) -> Vec<Run> {
    parse_markup(s, false).unwrap().remove(0)
}

fn width(s: &str) -> f64 {
    measure(s, 14.0, false).unwrap().width()
}

#[test]
fn box_drawing_falls_back_to_the_mono_face() {
    let b = measure("┌─┐│└┘", 14.0, false).unwrap();
    assert!(b.missing_glyphs().is_empty());
    assert_eq!(b.width(), width("`┌─┐│└┘`"));
    assert!((b.width() - 6.0 * 0.6 * 14.0).abs() < 1e-9, "{}", b.width());
    assert!(!outline(&runs("┌─┐"), 14.0, 0.0, 0.0).is_empty());
    assert!(color_layers(&runs("┌─┐"), 14.0, 0.0, 0.0).is_empty());
}

#[test]
fn code_falls_back_to_inter_and_bold_to_mono() {
    let code = measure("`₹`", 14.0, false).unwrap();
    assert!(code.missing_glyphs().is_empty());
    assert_eq!(code.width(), width("₹"));
    let bold = measure("**─**", 14.0, false).unwrap();
    assert!(bold.missing_glyphs().is_empty());
    assert_eq!(bold.width(), width("`─`"));
}

#[test]
fn characters_no_face_covers_are_still_missing() {
    let b = measure("日 한 ✘ 🚀 ┌", 14.0, false).unwrap();
    assert_eq!(b.missing_glyphs(), vec!['日', '한', '✘']);
}

#[test]
fn emoji_advance_one_em_in_any_run() {
    for s in ["🚀", "✅", "🔥", "✔", "**🚀**", "`🚀`"] {
        assert_eq!(width(s), 14.0, "{s}");
        assert!(measure(s, 14.0, false).unwrap().missing_glyphs().is_empty());
    }
    assert_eq!(width("a🚀b"), width("ab") + 14.0);
    assert_eq!(measure("🚀", 28.0, false).unwrap().width(), 28.0);
}

#[test]
fn sequences_form_one_ligature_glyph() {
    for s in [
        "👍🏽",
        "👩\u{200D}💻",
        "🇺🇸",
        "1\u{FE0F}\u{20E3}",
        "#\u{FE0F}\u{20E3}",
        "⚠\u{FE0F}",
        "❤\u{FE0F}",
    ] {
        assert_eq!(width(s), 14.0, "{s:?}");
        assert!(outline(&runs(s), 14.0, 0.0, 0.0).is_empty(), "{s:?}");
    }
    let layers = |s: &str| color_layers(&runs(s), 14.0, 0.0, 0.0);
    assert_ne!(layers("👍🏽"), layers("👍"));
    assert_ne!(layers("👩\u{200D}💻"), layers("👩"));
    assert_eq!(width("🇺🇸🇺"), 28.0);
}

#[test]
fn text_presentation_stays_in_the_text_face() {
    for s in ["⚠", "⚠\u{FE0E}", "1", "1\u{FE0F}", "©"] {
        assert!(color_layers(&runs(s), 14.0, 0.0, 0.0).is_empty(), "{s:?}");
        assert!(!outline(&runs(s), 14.0, 0.0, 0.0).is_empty(), "{s:?}");
    }
    assert_eq!(width("1\u{FE0F}"), width("1"));
}

#[test]
fn emoji_draw_as_colored_layers_after_the_pen() {
    let l = color_layers(&runs("ab🚀"), 14.0, 10.0, 20.0);
    assert!(!l.is_empty());
    assert!(l.iter().all(|l| l.color.is_some() && l.d.starts_with('M')));
    assert_eq!(l, color_layers(&runs("ab🚀"), 14.0, 10.0, 20.0));
    let shifted = color_layers(&runs("🚀"), 14.0, 10.0 + width("ab"), 20.0);
    assert_eq!(l, shifted);
    assert_eq!(
        outline(&runs("ab🚀"), 14.0, 10.0, 20.0),
        outline(&runs("ab"), 14.0, 10.0, 20.0)
    );
}

#[test]
fn only_emoji_ignores_spaces_and_needs_an_emoji() {
    assert!(only_emoji(&runs("🚀 ✅")));
    assert!(!only_emoji(&runs("🚀 a")));
    assert!(!only_emoji(&runs(" ")));
    assert!(!only_emoji(&runs("┌")));
}
