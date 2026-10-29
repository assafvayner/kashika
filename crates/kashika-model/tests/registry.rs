use std::collections::BTreeSet;

use kashika_model::{BUNDLED_ICONS, ICON_KEYWORDS, SHAPES, STYLES};

fn unique<'a>(names: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    names.into_iter().all(|n| seen.insert(n))
}

#[test]
fn every_builtin_shape_has_a_unique_name_and_keywords() {
    assert!(unique(SHAPES.iter().map(|s| s.name)));
    for s in SHAPES {
        assert!(!s.keywords.is_empty(), "shape `{}` has no keywords", s.name);
    }
}

#[test]
fn every_bundled_icon_has_keywords_and_every_keywords_entry_an_icon() {
    assert!(unique(BUNDLED_ICONS.iter().map(|(n, _)| *n)));
    assert!(unique(ICON_KEYWORDS.iter().map(|(n, _)| *n)));
    for (name, _) in BUNDLED_ICONS {
        let words = ICON_KEYWORDS.iter().find(|(n, _)| n == name);
        assert!(
            words.is_some_and(|(_, w)| !w.is_empty()),
            "icons/{name}.svg has no line in icons/keywords.toml"
        );
    }
    for (name, _) in ICON_KEYWORDS {
        assert!(
            BUNDLED_ICONS.iter().any(|(n, _)| n == name),
            "icons/keywords.toml names `{name}`, but there is no icons/{name}.svg"
        );
    }
}

#[test]
fn bundled_icons_keep_the_keywords_file_order() {
    let icons: Vec<&str> = BUNDLED_ICONS.iter().map(|(n, _)| *n).collect();
    let words: Vec<&str> = ICON_KEYWORDS.iter().map(|(n, _)| *n).collect();
    assert_eq!(icons, words);
}

#[test]
fn every_builtin_style_has_a_unique_name() {
    assert!(unique(STYLES.iter().map(|s| s.name)));
}
