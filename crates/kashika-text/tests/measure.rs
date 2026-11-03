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
    let runs = parse_markup("Hi **b** `c`", false).unwrap().remove(0);
    let d = outline(&runs, 14.0, 10.0, 20.0);
    let path = format!("{}/tests/fixtures/outline.txt", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
        std::fs::write(&path, format!("{d}\n")).unwrap();
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; bless with KASHIKA_BLESS=1"));
    assert_eq!(
        format!("{d}\n"),
        expected,
        "bless with KASHIKA_BLESS=1 if intended"
    );
    assert!(outline(&parse_markup(" ", false).unwrap()[0], 14.0, 0.0, 0.0).is_empty());
}

const TABLE: &[&str] = &[
    "",
    " ",
    "AV",
    "To",
    "Wa",
    "iiii",
    "MMMM",
    "a\tb",
    "**b** `c`",
    "🚀",
    "👩\u{200D}💻",
    "🇺🇸🇺",
    "1\u{FE0F}\u{20E3}",
    "⚠",
    "⚠\u{FE0F}",
    "┌─┐",
    "日本",
    "مرحبا",
    "café",
    "a\nb\nc",
];

/// The x of every point (on-curve and control) in SVG path data.
fn xs(d: &str) -> Vec<f64> {
    let nums: Vec<f64> = d
        .split(|c: char| c.is_ascii_alphabetic() || c == ' ')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .collect();
    nums.chunks(2).map(|p| p[0]).collect()
}

#[test]
fn ink_stays_inside_the_measured_box() {
    let (x, size) = (10.0, 14.0);
    let mut strings: Vec<String> = TABLE.iter().map(|s| s.to_string()).collect();
    strings.push("x".repeat(500));
    for s in &strings {
        for bold in [false, true] {
            let b = measure(s, size, bold).unwrap();
            for l in b.lines() {
                for ink in xs(&outline(&l.runs, size, x, 20.0)) {
                    assert!(
                        (x - 0.5..=x + l.width + 0.5).contains(&ink),
                        "{s:?} bold={bold}: ink at {ink} outside [{x}, {}]",
                        x + l.width
                    );
                }
                let layers = kashika_text::color_layers(&l.runs, size, x, 20.0);
                for layer in &layers {
                    let lx = xs(&layer.d);
                    let lo = lx.iter().copied().fold(f64::INFINITY, f64::min);
                    let hi = lx.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let pen = x + (((lo + hi) / 2.0 - x) / size).floor() * size;
                    assert!(
                        lo >= pen - 0.5 && hi <= pen + size + 0.5,
                        "{s:?}: emoji layer [{lo}, {hi}] outside [{pen}, {}]",
                        pen + size
                    );
                }
            }
        }
    }
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
