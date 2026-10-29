use assert_cmd::Command;
use predicates::prelude::*;

const TOPICS: &[&str] = &[
    "rules", "syntax", "anchors", "paths", "sizing", "text", "shapes", "styles", "recipes",
    "examples", "lints", "metrics", "cli", "limits",
];

fn kashika() -> Command {
    Command::cargo_bin("kashika").unwrap()
}

fn page(args: &[&str]) -> String {
    let out = kashika().arg("ref").args(args).output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{args:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn references_and_docs_are_aliases_of_ref() {
    let p = page(&["cli"]);
    for alias in ["references", "docs"] {
        let out = kashika().args([alias, "cli"]).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{alias}");
        assert_eq!(String::from_utf8(out.stdout).unwrap(), p, "{alias}");
    }
}

#[test]
fn default_page_has_the_required_parts_and_length() {
    let p = page(&[]);
    for heading in [
        "## The loop",
        "## Rules people get wrong",
        "## Syntax",
        "## Anchors and points",
        "## Line paths",
        "## Fixing diagnostics",
        "## Topics",
    ] {
        assert!(p.contains(heading), "missing {heading}");
    }
    for t in TOPICS {
        assert!(p.contains(&format!("| `{t}` |")), "topic list misses {t}");
    }
    let words = p
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count();
    assert!(
        (1500..=2500).contains(&words),
        "default page has {words} words"
    );
}

#[test]
fn recipes_page_has_every_recipe() {
    let p = page(&["recipes"]);
    for recipe in [
        "Staircase",
        "Wrap over the top",
        "Gap corridor",
        "Outside margin",
        "The comb",
        "Unavoidable crossings",
        "Fan-out via a junction",
        "Two lines on one side",
        "Self-loop",
        "Sequence diagrams",
        "Table cards",
        "Guides for rows and columns",
        "Graphs and timelines",
    ] {
        assert!(p.contains(recipe), "missing recipe {recipe}");
    }
}

#[test]
fn every_topic_has_a_page() {
    for t in TOPICS {
        assert!(!page(&[*t]).trim().is_empty(), "{t}");
    }
    assert!(
        page(&["examples"]).contains(
            &std::fs::read_to_string(format!(
                "{}/../../examples/upload/upload.kash",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap()
        )
    );
}

#[test]
fn every_diagnostic_code_has_a_page() {
    for (codes, severity) in [
        (kashika_diag::ERROR_CODES, "error"),
        (kashika_diag::LINT_CODES, "warn"),
    ] {
        for code in codes {
            let p = page(&[*code]);
            assert!(
                p.starts_with(&format!("# {code} ({severity})")),
                "{code}: {p}"
            );
            assert!(p.contains("Fix: "), "{code}");
        }
    }
}

#[test]
fn json_wraps_the_page() {
    let out = kashika()
        .args(["ref", "anchors", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["schema"], "kashika.ref/1");
    assert_eq!(v["topic"], "anchors");
    assert_eq!(v["topics"].as_array().unwrap().len(), TOPICS.len());
    assert!(
        v["markdown"]
            .as_str()
            .unwrap()
            .contains("## Anchors and points")
    );
    let out = kashika().args(["ref", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["topic"], serde_json::Value::Null);
}

#[test]
fn unknown_topic_is_a_usage_error_with_a_suggestion() {
    kashika()
        .args(["ref", "anchor"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("did you mean `anchors`?"));
    let out = kashika().args(["ref", "nope"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let err = String::from_utf8(out.stderr).unwrap();
    for t in TOPICS {
        assert!(err.contains(&format!("\n  {t} ")), "{err}");
    }
}

#[test]
fn metrics_are_generated_from_the_bundled_fonts() {
    let p = page(&["metrics"]);
    assert!(p.contains("| 14 |"), "{p}");
    assert!(p.contains("kashika measure"));
    let row = p.lines().find(|l| l.starts_with("| 14 |")).unwrap();
    let cells: Vec<f64> = row
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().parse().unwrap())
        .collect();
    assert!(cells[1] > 5.0 && cells[1] < 10.0, "{row}");
    assert!(cells[2] >= cells[1], "bold is not narrower: {row}");
    assert_eq!(cells[3], 17.5);
}

fn kash_blocks(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    for l in md.lines() {
        match (&mut cur, l) {
            (None, "```kash") => cur = Some(String::new()),
            (Some(_), "```") => out.extend(cur.take()),
            (Some(b), l) => {
                b.push_str(l);
                b.push('\n');
            }
            _ => {}
        }
    }
    out
}

#[test]
fn ref_examples_check_clean_and_are_formatted() {
    let mut blocks = kash_blocks(&page(&[]));
    for t in TOPICS {
        blocks.extend(kash_blocks(&page(&[*t])));
    }
    assert!(blocks.len() >= 9, "found {} blocks", blocks.len());
    assert_blocks_clean_and_formatted(blocks);
}

#[test]
fn skill_examples_check_clean_and_are_formatted() {
    let skill = std::fs::read_to_string(format!(
        "{}/../../skills/kashika/SKILL.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let blocks = kash_blocks(&skill);
    assert!(!blocks.is_empty(), "SKILL.md should have an example");
    assert_blocks_clean_and_formatted(blocks);
}

fn assert_blocks_clean_and_formatted(blocks: Vec<String>) {
    for b in blocks {
        let (scene, mut diags) = kashika_model::compile(&b);
        diags.extend(
            kashika_check::check(&scene)
                .into_iter()
                .filter(|d| !d.suppressed),
        );
        assert!(diags.is_empty(), "{b}\n{diags:#?}");
        assert_eq!(
            kashika_syntax::format(&b).unwrap(),
            b,
            "not in fmt form:\n{b}"
        );
    }
}

#[test]
fn shapes_page_lists_every_builtin_shape() {
    let p = page(&["shapes"]);
    for name in kashika_model::shape_names() {
        assert!(p.contains(&format!("\n| `{name}`")), "missing {name}");
    }
}

#[test]
fn skill_points_at_real_ref_topics() {
    let skill = std::fs::read_to_string(format!(
        "{}/../../skills/kashika/SKILL.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut checked = 0;
    for (i, _) in skill.match_indices("kashika ref ") {
        let topic: String = skill[i + "kashika ref ".len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if topic.is_empty() {
            continue;
        }
        checked += 1;
        kashika().args(["ref", &topic]).assert().code(0);
    }
    assert!(
        checked > 0,
        "SKILL.md should point at `kashika ref <topic>` pages"
    );
}

/// Every token inside the inline code spans and code blocks of `md`.
fn code_tokens(md: &str) -> std::collections::BTreeSet<&str> {
    let ticks = |s: &str| s.len() - s.trim_start_matches('`').len();
    let mut code = Vec::new();
    let mut rest = md;
    while let Some(i) = rest.find('`') {
        let n = ticks(&rest[i..]);
        let body = &rest[i + n..];
        let mut close = None;
        let mut from = 0;
        while let Some(j) = body[from..].find('`') {
            let k = ticks(&body[from + j..]);
            if k == n {
                close = Some(from + j);
                break;
            }
            from += j + k;
        }
        match close {
            Some(j) => {
                code.push(&body[..j]);
                rest = &body[j + n..];
            }
            None => rest = body,
        }
    }
    code.into_iter()
        .flat_map(|c| c.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')))
        .filter(|t| !t.is_empty())
        .collect()
}

#[test]
/// The spec and the `ref` pages name every built-in asset, so an agent that reads the topics
/// finds each one. The skill is a short entry point that sends agents to `kashika ref`, so it
/// lists none of them.
fn spec_and_ref_name_every_builtin_asset() {
    let shapes = kashika_model::shape_names().map(|n| ("shape", n));
    let icons = kashika_model::BUNDLED_ICONS
        .iter()
        .map(|(n, _)| ("icon", *n));
    let styles = kashika_model::STYLES.iter().map(|s| ("style", s.name));
    let markers = kashika_model::Marker::NAMES
        .iter()
        .map(|n| ("line end", *n));
    let all: Vec<(&str, &str)> = shapes.chain(icons).chain(styles).chain(markers).collect();
    let spec = std::fs::read_to_string(format!(
        "{}/../../docs/spec/language.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let pages: String = TOPICS.iter().map(|t| page(&[*t])).collect();
    for (doc, text) in [("docs/spec/language.md", spec), ("`kashika ref`", pages)] {
        let tokens = code_tokens(&text);
        for (kind, name) in &all {
            assert!(
                tokens.contains(name),
                "{doc} never names the built-in {kind} `{name}` in code"
            );
        }
    }
}
