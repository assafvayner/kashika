use assert_cmd::Command;

const STRINGS: &[&str] = &[
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

const SIZES: [&str; 3] = ["10", "14", "28"];
const WEIGHTS: [&str; 2] = ["normal", "bold"];

fn strings() -> Vec<String> {
    let mut v: Vec<String> = STRINGS.iter().map(|s| s.to_string()).collect();
    v.insert(19, "x".repeat(500));
    v
}

/// The `measure` argument for `s`: the CLI unescapes `\n`, `\"` and `\\`.
fn arg(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

/// `s` with backslashes, tabs, newlines, other controls and default-ignorables escaped,
/// so the table stays one row per line and shows invisible characters.
fn show(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || kashika_text::is_default_ignorable(c) => {
                out.push_str(&format!("\\u{{{:x}}}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out
}

fn table() -> String {
    let strings = strings();
    let mut out = String::from("size\tweight\tstring\twidth\theight\tlines\n");
    for size in SIZES {
        for weight in WEIGHTS {
            let mut args = vec![
                "measure".to_string(),
                "--format".into(),
                "json".into(),
                "--size".into(),
                size.into(),
                "--weight".into(),
                weight.into(),
            ];
            args.extend(strings.iter().map(|s| arg(s)));
            let output = Command::cargo_bin("kashika")
                .unwrap()
                .args(&args)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let results = v["results"].as_array().unwrap();
            assert_eq!(results.len(), strings.len());
            for (s, r) in strings.iter().zip(results) {
                assert_eq!(r["text"].as_str().unwrap(), arg(s));
                out.push_str(&format!(
                    "{size}\t{weight}\t{}\t{}\t{}\t{}\n",
                    show(s),
                    r["width"],
                    r["height"],
                    r["lines"]
                ));
            }
        }
    }
    out
}

#[test]
fn measurements_match_the_table() {
    let path = format!("{}/tests/fixtures/measure.txt", env!("CARGO_MANIFEST_DIR"));
    let actual = table();
    if std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1") {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; bless with KASHIKA_BLESS=1"));
    if actual != expected {
        let diff = similar::TextDiff::from_lines(&expected, &actual)
            .unified_diff()
            .header("expected", "actual")
            .to_string();
        panic!("measure.txt changed; bless with KASHIKA_BLESS=1 if intended\n{diff}");
    }
}
