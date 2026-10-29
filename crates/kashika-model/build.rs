//! Generates the bundled icon tables from `icons/*.svg` and `icons/keywords.toml`.

use std::fmt::Write;
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut svgs: Vec<String> = std::fs::read_dir(&dir)
        .expect("icons/ is readable")
        .map(|e| e.expect("icons/ entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "svg"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    svgs.sort();

    let toml = std::fs::read_to_string(dir.join("keywords.toml")).expect("icons/keywords.toml");
    let keywords: Vec<(String, Vec<String>)> = toml.lines().filter_map(parse_line).collect();

    let position = |name: &str| keywords.iter().position(|(n, _)| n == name);
    svgs.sort_by_key(|n| position(n).unwrap_or(usize::MAX));

    let mut out = String::from(
        "/// The neutral icon set shipped with Kashika, addressed by bare name (`icon=\"server\"`).\n\
         pub const BUNDLED_ICONS: &[(&str, &str)] = &[\n",
    );
    for name in &svgs {
        writeln!(
            out,
            "    ({name:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/icons/{name}.svg\"))),"
        )
        .unwrap();
    }
    out.push_str(
        "];\n\n/// Search keywords per bundled icon, for `kashika icons --search`.\n\
         pub const ICON_KEYWORDS: &[(&str, &[&str])] = &[\n",
    );
    for (name, words) in &keywords {
        writeln!(out, "    ({name:?}, &{words:?}),").unwrap();
    }
    out.push_str("];\n");

    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("icons.rs");
    std::fs::write(dest, out).expect("write icons.rs");
}

/// Parses `name = ["a", "b"]`; blank lines and `#` comments give `None`.
fn parse_line(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (name, list) = line
        .split_once('=')
        .unwrap_or_else(|| panic!("icons/keywords.toml: expected `name = [...]`: {line}"));
    let list = list
        .trim()
        .strip_prefix('[')
        .and_then(|l| l.strip_suffix(']'))
        .unwrap_or_else(|| panic!("icons/keywords.toml: expected a [...] list: {line}"));
    let words = list
        .split(',')
        .map(|w| w.trim().trim_matches('"').to_string())
        .filter(|w| !w.is_empty())
        .collect();
    Some((name.trim().to_string(), words))
}
