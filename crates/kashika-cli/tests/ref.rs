use assert_cmd::Command;
use predicates::prelude::*;

const TOPICS: &[&str] = &[
    "syntax", "anchors", "paths", "shapes", "styles", "metrics", "lints", "examples", "cli",
];

/// Error codes from spec §7, plus `text-conflict` from §6. Warnings aren't reported by this
/// build, so they have no pages.
const ERROR_CODES: &[&str] = &[
    "parse",
    "unknown-id",
    "unknown-anchor",
    "duplicate-id",
    "duplicate-style",
    "placement-cycle",
    "label-overflow",
    "text-conflict",
    "not-ortho",
    "degenerate-curve",
    "zero-length",
    "attr",
    "unknown-style",
    "size",
    "diagram",
    "version",
    "unsupported",
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
    let words = p.split_whitespace().count();
    assert!(
        (1500..=2300).contains(&words),
        "default page has {words} words"
    );
}

#[test]
fn every_topic_has_a_page() {
    for t in TOPICS {
        assert!(!page(&[*t]).trim().is_empty(), "{t}");
    }
    assert!(page(&["examples"]).contains(
        &std::fs::read_to_string(format!(
            "{}/../../examples/upload.kash",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    ));
}

#[test]
fn every_error_code_has_a_page() {
    for code in ERROR_CODES {
        let p = page(&[*code]);
        assert!(p.starts_with(&format!("# {code} (")), "{code}: {p}");
        assert!(p.contains("Fix: "), "{code}");
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
    assert!(v["markdown"]
        .as_str()
        .unwrap()
        .contains("## Anchors and points"));
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
    for b in blocks {
        let (_, diags) = kashika_model::compile(&b);
        assert!(diags.is_empty(), "{b}\n{diags:#?}");
        assert_eq!(
            kashika_syntax::format(&b).unwrap(),
            b,
            "not in fmt form:\n{b}"
        );
    }
}
