use kashika_text::{covered_scripts, is_default_ignorable, measure, outline, parse_markup};

#[test]
fn widths_come_from_the_bundled_fonts() {
    let b = measure("API", 14.0, false).unwrap();
    assert!((b.width() - 22.3604).abs() < 1e-3, "{}", b.width());
    assert_eq!(b.height(), 17.5);
    let bold = measure("API", 14.0, true).unwrap();
    assert!((bold.width() - 23.4541).abs() < 1e-3, "{}", bold.width());
    let code = measure("`API`", 14.0, false).unwrap();
    assert!((code.width() - 25.2).abs() < 1e-9, "{}", code.width());
}

#[test]
fn width_scales_with_size_and_lines_stack() {
    let a = measure("Postgres", 14.0, false).unwrap();
    let b = measure("Postgres", 28.0, false).unwrap();
    assert!((b.width() - 2.0 * a.width()).abs() < 1e-9);
    let two = measure("Postgres\nAPI", 12.0, false).unwrap();
    assert!((two.width() - 50.3262).abs() < 1e-3);
    assert_eq!(two.height(), 30.0);
    assert_eq!(two.lines().len(), 2);
}

#[test]
fn missing_glyphs_use_notdef_width() {
    let b = measure("\u{10FFFD}", 14.0, false).unwrap();
    assert!((b.width() - 1344.0 * 14.0 / 2048.0).abs() < 1e-9);
}

#[test]
fn unclosed_markup_fails_measurement() {
    assert_eq!(measure("**API", 14.0, false).unwrap_err().marker, "**");
}

#[test]
fn outline_is_deterministic_path_data() {
    let runs = parse_markup("Hi", false).unwrap().remove(0);
    let d = outline(&runs, 14.0, 10.0, 20.0);
    assert!(d.starts_with('M'), "{d}");
    assert!(d.contains('Z'));
    assert_eq!(d, outline(&runs, 14.0, 10.0, 20.0));
    assert!(outline(&parse_markup(" ", false).unwrap()[0], 14.0, 0.0, 0.0).is_empty());
}

#[test]
fn missing_glyphs_lists_each_character_once_in_order() {
    let b = measure("a 日 **本日** `é`", 14.0, false).unwrap();
    assert_eq!(b.missing_glyphs(), vec!['日', '本']);
    assert!(
        measure("plain é ü", 14.0, false)
            .unwrap()
            .missing_glyphs()
            .is_empty()
    );
}

#[test]
fn tabs_measure_as_four_spaces_and_are_not_missing() {
    let t = measure("a\tb", 14.0, false).unwrap();
    let s = measure("a    b", 14.0, false).unwrap();
    assert_eq!(t.width(), s.width());
    assert!(t.missing_glyphs().is_empty());
}

#[test]
fn default_ignorable_characters_are_zero_width_and_never_missing() {
    let plain = measure("ab", 14.0, false).unwrap();
    for c in [
        '\u{200D}',
        '\u{200C}',
        '\u{FE0F}',
        '\u{E0100}',
        '\u{200B}',
        '\u{2060}',
        '\u{FEFF}',
        '\u{00AD}',
    ] {
        assert!(is_default_ignorable(c), "{c:?}");
        let text = format!("a{c}b");
        let b = measure(&text, 14.0, false).unwrap();
        assert_eq!(b.width(), plain.width(), "{c:?}");
        assert!(b.missing_glyphs().is_empty(), "{c:?}");
        let runs = parse_markup(&text, false).unwrap().remove(0);
        let ab = parse_markup("ab", false).unwrap().remove(0);
        assert_eq!(outline(&runs, 14.0, 0.0, 0.0), outline(&ab, 14.0, 0.0, 0.0));
    }
    assert!(!is_default_ignorable('a'));
    assert!(!is_default_ignorable('\u{2010}'));
}

#[test]
fn covered_scripts_come_from_the_fonts() {
    assert_eq!(
        covered_scripts(),
        ["Latin", "Greek", "Cyrillic", "box drawing", "emoji"]
    );
}
