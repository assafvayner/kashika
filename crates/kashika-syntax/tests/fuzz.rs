use std::path::Path;

use kashika_syntax::{format, parse};
use proptest::prelude::*;

/// Token spellings, comments and line breaks the generated sources are built from.
const ALPHABET: &[&str] = &[
    "diagram",
    "style",
    "box",
    "group",
    "junction",
    "text",
    "line",
    "arrow",
    "import",
    "shape",
    "image",
    "path",
    "outline",
    "text-area",
    "at",
    "size",
    "auto",
    "as",
    "a",
    "b",
    "a-b",
    "_c",
    "top",
    "left",
    "h",
    "v",
    "x",
    "y",
    "hv",
    "vh",
    "vhv",
    "hvh",
    "curve",
    "via",
    "right",
    "ortho",
    "end",
    "pin",
    "label",
    "allow",
    "cross",
    "fill",
    "0",
    "1",
    "-2",
    "0.5",
    "10.50",
    "1e5",
    "\"s\"",
    "\"a\\\"b\"",
    "\"\\q\"",
    "\"open",
    "#fff",
    "#12345678",
    "#zz",
    "#",
    "(",
    ")",
    "{",
    "}",
    ",",
    "=",
    "->",
    ".",
    "+",
    "-",
    ":",
    "@",
    "// c",
    "/* c */",
    "/* x\ny */",
    "/*",
    "*/",
    "\n",
    "\r\n",
    "\r",
    "\t",
    "\u{feff}",
    "\u{a0}",
    "é",
];

const SEPARATORS: &[&str] = &["", " ", "  ", "\n", "\n  "];

const MUTATION_CHARS: &[char] = &[
    '{', '}', '(', ')', '"', '-', '>', '/', '*', '#', '\n', '\r', '\t', ' ', '=', ',', '.', '@',
    ':', '+', 'a', '0',
];

fn golden_inputs() -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let input = dir.join("input.kash");
        if input.is_file() {
            out.push(std::fs::read_to_string(input).unwrap());
            return;
        }
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect();
        entries.sort();
        for p in entries {
            walk(&p, out);
        }
    }
    let mut out = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden"),
        &mut out,
    );
    assert!(!out.is_empty());
    out
}

/// `parse` returns, `format` refuses the input or is idempotent on its output, and the
/// formatter's token guard never fires.
fn holds(src: &str) -> Result<(), TestCaseError> {
    let (_, diags) = parse(src);
    prop_assert!(
        !diags
            .iter()
            .any(|d| d.message.starts_with("internal error")),
        "parse: {diags:#?}"
    );
    match format(src) {
        Ok(once) => {
            let twice = format(&once);
            prop_assert_eq!(twice.as_deref(), Ok(once.as_str()), "fmt is not idempotent");
        }
        Err(diags) => prop_assert!(
            !diags
                .iter()
                .any(|d| d.message.starts_with("internal error")),
            "fmt guard fired: {diags:#?}"
        ),
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum Mutation {
    Insert(usize, char),
    Delete(usize),
    Replace(usize, char),
}

fn mutation() -> impl Strategy<Value = Mutation> {
    let c = || proptest::sample::select(MUTATION_CHARS);
    prop_oneof![
        (any::<usize>(), c()).prop_map(|(i, c)| Mutation::Insert(i, c)),
        any::<usize>().prop_map(Mutation::Delete),
        (any::<usize>(), c()).prop_map(|(i, c)| Mutation::Replace(i, c)),
    ]
}

fn mutate(src: &str, mutations: &[Mutation]) -> String {
    let mut chars: Vec<char> = src.chars().collect();
    for m in mutations {
        match *m {
            Mutation::Insert(i, c) => chars.insert(i % (chars.len() + 1), c),
            Mutation::Delete(i) if !chars.is_empty() => {
                chars.remove(i % chars.len());
            }
            Mutation::Replace(i, c) if !chars.is_empty() => {
                let n = chars.len();
                chars[i % n] = c;
            }
            _ => {}
        }
    }
    chars.into_iter().collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn random_token_sequences(
        toks in proptest::collection::vec(
            (proptest::sample::select(ALPHABET), proptest::sample::select(SEPARATORS)),
            0..=60,
        )
    ) {
        let src: String = toks.iter().flat_map(|(t, s)| [*t, *s]).collect();
        holds(&src)?;
    }

    #[test]
    fn mutated_golden_inputs(
        src in proptest::sample::select(golden_inputs()),
        mutations in proptest::collection::vec(mutation(), 1..=5),
    ) {
        holds(&mutate(&src, &mutations))?;
    }
}
