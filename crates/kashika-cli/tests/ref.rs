use assert_cmd::Command;
use predicates::prelude::*;

const TOPICS: &[&str] = &[
    "syntax", "anchors", "paths", "shapes", "styles", "metrics", "lints", "examples", "cli",
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
        "## Five rules people get wrong",
        "## Syntax",
        "## Anchors and points",
        "## Line paths",
        "## Diagnostics",
        "## Routing recipes",
        "## Worked example",
        "## Limits of a clean check",
        "## Not supported in this build",
    ] {
        assert!(p.contains(heading), "missing {heading}");
    }
    for recipe in [
        "Staircase",
        "Wrap over the top",
        "Gap corridor",
        "Outside margin",
        "Fan-out via a junction",
        "The comb",
        "Unavoidable crossings",
    ] {
        assert!(p.contains(recipe), "missing recipe {recipe}");
    }
    let words = p
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count();
    assert!(
        (1500..=3000).contains(&words),
        "default page has {words} words"
    );
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
    assert!(blocks.len() >= 2, "found {} blocks", blocks.len());
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
    for name in kashika_model::BUILTIN_SHAPES {
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
