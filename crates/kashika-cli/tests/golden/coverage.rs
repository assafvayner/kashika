use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kashika_diag::{ERROR_CODES, LINT_CODES};
use libtest_mimic::{Failed, Trial};

use crate::case::{self, Case, Suite};

/// Codes no build reports yet, with what a case can show instead.
const NOT_REPORTED: &[(&str, &str)] = &[];

struct Corpus {
    cases: Vec<Case>,
    /// Code -> cases whose expected diagnostics contain it.
    codes: BTreeMap<String, BTreeSet<String>>,
}

fn spec_text() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/language.md"),
    )
    .expect("spec is readable")
}

/// `## 3a. Shapes, images and imports` -> (`3a`, `Shapes, images and imports`).
fn spec_sections(spec: &str) -> Vec<(String, String)> {
    spec.lines()
        .filter_map(|l| l.strip_prefix("## "))
        .filter_map(|h| h.split_once(". "))
        .map(|(id, title)| (id.to_string(), title.to_string()))
        .collect()
}

/// The backticked codes in the first column of a §7 table, between `### <title>` and the
/// next `###` heading.
fn spec_codes(spec: &str, title: &str) -> BTreeSet<String> {
    let mut on = false;
    let mut codes = BTreeSet::new();
    for line in spec.lines() {
        if line.starts_with("### ") {
            on = line == format!("### {title}");
            continue;
        }
        if !on || !line.starts_with("| `") {
            continue;
        }
        let cell = line.split('|').nth(1).unwrap_or("");
        codes.extend(cell.split('`').skip(1).step_by(2).map(str::to_string));
    }
    codes
}

fn load() -> Result<Corpus, String> {
    let root = case::golden_root();
    let mut cases = Vec::new();
    let mut codes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for dir in case::discover(&root) {
        let c = Case::load(&root, &dir)?;
        let diags = std::fs::read_to_string(dir.join("diagnostics.txt")).unwrap_or_default();
        for line in diags.lines() {
            let Some(rest) = line
                .strip_prefix("error[")
                .or_else(|| line.strip_prefix("warn["))
            else {
                continue;
            };
            if let Some((code, _)) = rest.split_once(']') {
                codes
                    .entry(code.to_string())
                    .or_default()
                    .insert(c.name.clone());
            }
        }
        cases.push(c);
    }
    Ok(Corpus { cases, codes })
}

fn all_codes() -> Vec<(&'static str, &'static str)> {
    ERROR_CODES
        .iter()
        .map(|c| (*c, "error"))
        .chain(LINT_CODES.iter().map(|c| (*c, "warn")))
        .collect()
}

pub fn trials() -> Vec<Trial> {
    vec![
        Trial::test("coverage/code-lists-match-spec", code_lists_match_spec),
        Trial::test("coverage/every-code-has-a-case", every_code_has_a_case),
        Trial::test(
            "coverage/every-section-has-a-case",
            every_section_has_a_case,
        ),
        Trial::test(
            "coverage/builtin-shapes-and-icons",
            builtin_shapes_and_icons,
        ),
        Trial::test("coverage/map", coverage_map),
    ]
}

fn code_lists_match_spec() -> Result<(), Failed> {
    let spec = spec_text();
    let errors: BTreeSet<String> = ERROR_CODES.iter().map(|c| c.to_string()).collect();
    let warns: BTreeSet<String> = LINT_CODES.iter().map(|c| c.to_string()).collect();
    let spec_errors = spec_codes(&spec, "Errors");
    let spec_warns = spec_codes(&spec, "Warnings");
    let mut problems = Vec::new();
    if errors != spec_errors {
        problems.push(format!(
            "ERROR_CODES and the spec's Errors table differ: only in code {:?}, only in spec {:?}",
            errors.difference(&spec_errors).collect::<Vec<_>>(),
            spec_errors.difference(&errors).collect::<Vec<_>>()
        ));
    }
    if warns != spec_warns {
        problems.push(format!(
            "LINT_CODES and the spec's Warnings table differ: only in code {:?}, only in spec {:?}",
            warns.difference(&spec_warns).collect::<Vec<_>>(),
            spec_warns.difference(&warns).collect::<Vec<_>>()
        ));
    }
    done(problems)
}

fn every_code_has_a_case() -> Result<(), Failed> {
    let corpus = load()?;
    let mut problems = Vec::new();
    for (code, _) in all_codes() {
        let reported = corpus.codes.contains_key(code);
        match NOT_REPORTED.iter().find(|(c, _)| *c == code) {
            Some(_) if reported => problems.push(format!(
                "`{code}` is reported now: remove it from NOT_REPORTED in coverage.rs"
            )),
            Some(_) if allow_cases(&corpus, code).is_empty() => {
                problems.push(format!("no case accepts `allow={code}`"))
            }
            Some(_) => {}
            None if !reported => problems.push(format!(
                "no golden case reports `{code}`; add one under tests/golden/"
            )),
            None => {}
        }
    }
    for code in corpus.codes.keys() {
        if !all_codes().iter().any(|(c, _)| c == code) {
            problems.push(format!(
                "`{code}` appears in a diagnostics.txt but in neither ERROR_CODES nor LINT_CODES"
            ));
        }
    }
    done(problems)
}

fn allow_cases(corpus: &Corpus, code: &str) -> Vec<String> {
    corpus
        .cases
        .iter()
        .filter(|c| {
            c.src
                .split("allow=")
                .skip(1)
                .any(|rest| rest.split_whitespace().next().unwrap_or("").contains(code))
        })
        .map(|c| c.name.clone())
        .collect()
}

fn every_section_has_a_case() -> Result<(), Failed> {
    let corpus = load()?;
    let sections = spec_sections(&spec_text());
    let mut problems = Vec::new();
    for c in &corpus.cases {
        for s in &c.spec {
            if !sections.iter().any(|(id, _)| id == s) {
                problems.push(format!("{}: `// spec: {s}` names no spec section", c.name));
            }
        }
    }
    for (id, title) in &sections {
        if !corpus.cases.iter().any(|c| c.spec.contains(id)) {
            problems.push(format!("no case covers §{id} {title}"));
        }
    }
    done(problems)
}

fn builtin_shapes_and_icons() -> Result<(), Failed> {
    let corpus = load()?;
    let render: Vec<&Case> = corpus
        .cases
        .iter()
        .filter(|c| c.suite == Suite::Render)
        .collect();
    let mut problems = Vec::new();
    for shape in kashika_model::BUILTIN_SHAPES {
        let needle = format!("shape={shape}\n");
        let needle_sp = format!("shape={shape} ");
        if !render
            .iter()
            .any(|c| c.src.contains(&needle) || c.src.contains(&needle_sp))
        {
            problems.push(format!("no render case draws built-in shape `{shape}`"));
        }
    }
    for (icon, _) in kashika_model::BUNDLED_ICONS {
        if !render
            .iter()
            .any(|c| c.src.contains(&format!("icon=\"{icon}\"")))
        {
            problems.push(format!("no render case draws bundled icon `{icon}`"));
        }
    }
    done(problems)
}

fn map_path() -> PathBuf {
    case::golden_root().join("COVERAGE.md")
}

fn render_map(corpus: &Corpus) -> String {
    let list = |names: &mut dyn Iterator<Item = &String>| {
        names
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = String::from(
        "# Golden test coverage\n\nGenerated by `KASHIKA_BLESS=1 cargo test -p kashika-cli --test golden -- coverage/map`; do not edit.\n\n## Spec sections\n\n| section | cases |\n|---|---|\n",
    );
    for (id, title) in spec_sections(&spec_text()) {
        let mut names = corpus
            .cases
            .iter()
            .filter(|c| c.spec.contains(&id))
            .map(|c| &c.name);
        out.push_str(&format!("| §{id} {title} | {} |\n", list(&mut names)));
    }
    out.push_str("\n## Diagnostic codes\n\n| code | severity | cases |\n|---|---|---|\n");
    for (code, severity) in all_codes() {
        let cases = match NOT_REPORTED.iter().find(|(c, _)| *c == code) {
            Some((_, why)) => {
                let allowed = allow_cases(corpus, code);
                if allowed.is_empty() {
                    why.to_string()
                } else {
                    format!("{why} in {}", list(&mut allowed.iter()))
                }
            }
            None => corpus
                .codes
                .get(code)
                .map(|s| list(&mut s.iter()))
                .unwrap_or_default(),
        };
        out.push_str(&format!("| `{code}` | {severity} | {cases} |\n"));
    }
    out
}

fn coverage_map() -> Result<(), Failed> {
    let corpus = load()?;
    let fresh = render_map(&corpus);
    if std::env::var_os("KASHIKA_BLESS").is_some_and(|v| v == "1") {
        std::fs::write(map_path(), &fresh).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let committed = std::fs::read_to_string(map_path()).unwrap_or_default();
    if committed == fresh {
        return Ok(());
    }
    Err(format!(
        "tests/golden/COVERAGE.md is stale:\n{}\nregenerate with: KASHIKA_BLESS=1 cargo test -p kashika-cli --test golden -- coverage/map",
        similar::TextDiff::from_lines(&committed, &fresh)
            .unified_diff()
            .header("committed", "fresh")
    )
    .into())
}

fn done(problems: Vec<String>) -> Result<(), Failed> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n").into())
    }
}
