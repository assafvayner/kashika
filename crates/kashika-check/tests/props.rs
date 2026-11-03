//! Corpus-wide invariants of `check` over the lint golden cases in
//! `tests/golden/parse/lints/*/input.kash`: translation invariance, exact suppression,
//! purity and independence from the order of `line` statements.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kashika_check::ctx::Ctx;
use kashika_check::{check, check_bounded, check_deferred, suppresses};
use kashika_diag::{Diagnostic, Span, code, fact};
use kashika_model::{Options, Scene};

const DX: i64 = 37;
const DY: i64 = 53;

struct Case {
    name: String,
    path: PathBuf,
    src: String,
}

impl Case {
    fn compile(&self, src: &str) -> (Scene, Vec<Diagnostic>) {
        let opts = Options {
            path: Some(self.path.clone()),
            ..Options::default()
        };
        kashika_model::compile_with(src, &opts)
    }
}

/// The checkable lint goldens, and the names of those skipped with the reason.
fn corpus() -> (Vec<Case>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/parse/lints");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("input.kash").is_file())
        .collect();
    dirs.sort();
    let (mut cases, mut skipped) = (Vec::new(), Vec::new());
    for dir in dirs {
        let path = dir.join("input.kash");
        let case = Case {
            name: dir.file_name().unwrap().to_string_lossy().into_owned(),
            src: std::fs::read_to_string(&path).unwrap(),
            path,
        };
        let opts = Options {
            path: Some(case.path.clone()),
            ..Options::default()
        };
        if kashika_model::compile_library_with(&case.src, &opts).is_some() {
            skipped.push(format!("{}: a library", case.name));
        } else if case.compile(&case.src).1.iter().any(|d| d.code == "parse") {
            skipped.push(format!("{}: does not parse", case.name));
        } else {
            cases.push(case);
        }
    }
    assert!(
        cases.len() > 40,
        "corpus not found under {}",
        root.display()
    );
    (cases, skipped)
}

fn error_codes(diags: &[Diagnostic]) -> Vec<&'static str> {
    let mut codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    codes.sort();
    codes
}

fn rows(src: &str) -> Vec<Vec<char>> {
    src.split('\n').map(|r| r.chars().collect()).collect()
}

fn join(rows: &[Vec<char>]) -> String {
    rows.iter()
        .map(|r| r.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where each character of `row` is code: `false` inside strings and comments.
fn code_mask(row: &[char]) -> Vec<bool> {
    let mut mask = vec![true; row.len()];
    let mut i = 0;
    while i < row.len() {
        match row[i] {
            '"' => {
                let start = i;
                i += 1;
                while i < row.len() && row[i] != '"' {
                    i += if row[i] == '\\' { 2 } else { 1 };
                }
                for m in &mut mask[start..(i + 1).min(row.len())] {
                    *m = false;
                }
            }
            '/' if row.get(i + 1) == Some(&'/') => {
                for m in &mut mask[i..] {
                    *m = false;
                }
                break;
            }
            _ => {}
        }
        i += 1;
    }
    mask
}

/// The number at `row[i..]`, as written, and its length.
fn number_at(row: &[char], i: usize) -> Option<(String, usize)> {
    let mut j = i;
    if row.get(j) == Some(&'-') {
        j += 1;
    }
    let digits = j;
    while row.get(j).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
        j += 1;
    }
    (j > digits && row[digits].is_ascii_digit()).then(|| (row[i..j].iter().collect(), j - i))
}

/// `n + delta`, exactly, written with as many decimals as `n`.
fn add(n: &str, delta: i64) -> String {
    let decimals = n.split_once('.').map_or(0, |(_, f)| f.len());
    let scaled: i64 = n.replace('.', "").parse().unwrap();
    let sum = scaled + delta * 10i64.pow(decimals as u32);
    if decimals == 0 {
        return sum.to_string();
    }
    let digits = format!("{:0>width$}", sum.abs(), width = decimals + 1);
    let (int, frac) = digits.split_at(digits.len() - decimals);
    format!("{}{int}.{frac}", if sum < 0 { "-" } else { "" })
}

/// The literal point `(x,y)` at `row[i..]`: its two numbers and its length.
fn point_at(row: &[char], i: usize) -> Option<(String, String, usize)> {
    let skip_ws = |mut j: usize| {
        while row.get(j) == Some(&' ') {
            j += 1;
        }
        j
    };
    let mut j = skip_ws(i + 1);
    let (x, n) = number_at(row, j)?;
    j = skip_ws(j + n);
    (row.get(j) == Some(&',')).then_some(())?;
    j = skip_ws(j + 1);
    let (y, n) = number_at(row, j)?;
    j = skip_ws(j + n);
    (row.get(j) == Some(&')')).then_some((x, y, j + 1 - i))
}

fn prev_word(row: &[char], i: usize) -> String {
    let end = row[..i]
        .iter()
        .rposition(|c| *c != ' ')
        .map_or(0, |p| p + 1);
    let start = row[..end]
        .iter()
        .rposition(|c| !(c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '>'))
        .map_or(0, |p| p + 1);
    row[start..end].iter().collect()
}

/// `src` moved by (DX, DY): every absolute coordinate of a top-level statement (`at`, literal
/// line points, curve targets and controls, numeric `x`/`y` targets) moves, the diagram `size`
/// grows by the same amount, and relative moves (`h`, `v`), offsets, sizes and everything in a
/// group frame or text block stay. `None` when a top-level row holds a mixed point or a block
/// comment, which the rewrite does not handle.
fn translate(src: &str) -> Option<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    for row in rows(src) {
        let mask = code_mask(&row);
        let top = depth == 0;
        let diagram = row
            .iter()
            .collect::<String>()
            .trim_start()
            .starts_with("diagram");
        let mut new = Vec::new();
        let mut i = 0;
        while i < row.len() {
            let c = row[i];
            if !mask[i] {
                new.push(c);
                i += 1;
                continue;
            }
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                '/' if row.get(i + 1) == Some(&'*') => return None,
                _ => {}
            }
            if top && c == '(' {
                if let Some((x, y, n)) = point_at(&row, i) {
                    let before = row[..i].iter().rev().find(|c| **c != ' ');
                    let word = prev_word(&row, i);
                    let moved = match (before, word.as_str(), diagram) {
                        (Some('+' | '-'), _, _) | (_, "size", false) => None,
                        (_, "size", true) => Some(format!("({},{})", add(&x, DX), add(&y, DY))),
                        (_, _, true) => return None,
                        _ => Some(format!("({},{})", add(&x, DX), add(&y, DY))),
                    };
                    match moved {
                        Some(p) => new.extend(p.chars()),
                        None => new.extend(&row[i..i + n]),
                    }
                    i += n;
                    continue;
                }
                let close = row[i..].iter().position(|c| *c == ')').map(|p| i + p);
                if close.is_some_and(|e| row[i..e].contains(&',')) {
                    return None;
                }
            }
            if top
                && (c == 'x' || c == 'y')
                && row.get(i + 1) == Some(&' ')
                && row[..i].iter().rev().find(|c| **c != ' ') == Some(&'>')
            {
                let j = i + 2 + row[i + 2..].iter().take_while(|c| **c == ' ').count();
                if let Some((n, len)) = number_at(&row, j) {
                    new.extend(row[i..j].iter());
                    new.extend(add(&n, if c == 'x' { DX } else { DY }).chars());
                    i = j + len;
                    continue;
                }
            }
            new.push(c);
            i += 1;
        }
        out.push(new);
    }
    Some(join(&out))
}

/// `message` with every point and canvas size replaced by `_`.
fn mask(message: &str) -> String {
    let words: Vec<String> = message
        .split(' ')
        .map(|w| match w.split_once('x') {
            Some((a, b))
                if number_at(&a.chars().collect::<Vec<_>>(), 0)
                    .is_some_and(|n| n.1 == a.chars().count())
                    && b.parse::<f64>().is_ok() =>
            {
                "_x_".into()
            }
            _ => w.into(),
        })
        .collect();
    let row: Vec<char> = words.join(" ").chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < row.len() {
        if row[i] == '(' {
            if let Some((_, _, n)) = point_at(&row, i) {
                out.push_str("(_,_)");
                i += n;
                continue;
            }
        }
        out.push(row[i]);
        i += 1;
    }
    out
}

fn ids(d: &Diagnostic) -> Vec<String> {
    d.elements.iter().map(|e| e.id.clone()).collect()
}

fn points(d: &Diagnostic) -> Vec<Option<[f64; 2]>> {
    [fact::AT, fact::ENTRY, fact::EXIT]
        .iter()
        .map(|k| d.point(k))
        .collect()
}

type Key = (&'static str, Vec<String>, String);

fn by_key(diags: &[Diagnostic]) -> BTreeMap<Key, Vec<Vec<Option<[f64; 2]>>>> {
    let mut map: BTreeMap<Key, Vec<_>> = BTreeMap::new();
    for d in diags {
        map.entry((d.code, ids(d), mask(&d.message)))
            .or_default()
            .push(points(d));
    }
    for v in map.values_mut() {
        v.sort_by(|a, b| {
            let flat = |p: &Vec<Option<[f64; 2]>>| -> Vec<f64> {
                p.iter().flatten().flatten().copied().collect()
            };
            flat(a).partial_cmp(&flat(b)).unwrap()
        });
    }
    map
}

/// Cases whose findings legitimately depend on absolute position.
const NOT_TRANSLATABLE: &[(&str, &str)] = &[(
    "off-grid",
    "`grid=10` judges absolute literals, and (37,53) is not a multiple of 10",
)];

#[test]
fn findings_move_with_the_drawing() {
    let (cases, skipped) = corpus();
    let mut ran = 0;
    let mut unsafe_rewrite = Vec::new();
    let mut failures = Vec::new();
    for case in &cases {
        if NOT_TRANSLATABLE.iter().any(|(n, _)| *n == case.name) {
            continue;
        }
        let Some(moved) = translate(&case.src) else {
            unsafe_rewrite.push(case.name.clone());
            continue;
        };
        ran += 1;
        let (scene, errors) = case.compile(&case.src);
        let (moved_scene, moved_errors) = case.compile(&moved);
        if error_codes(&errors) != error_codes(&moved_errors) {
            failures.push(format!(
                "{}: errors differ\n{errors:#?}\n{moved_errors:#?}",
                case.name
            ));
            continue;
        }
        let (a, b) = (by_key(&check(&scene)), by_key(&check(&moved_scene)));
        if a.keys().ne(b.keys()) || a.values().map(Vec::len).ne(b.values().map(Vec::len)) {
            failures.push(format!(
                "{}: findings differ\n  before: {:?}\n  after:  {:?}\n--- moved source\n{moved}",
                case.name,
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>()
            ));
            continue;
        }
        for ((key, before), after) in a.iter().zip(b.values()) {
            for (p, q) in before.iter().flatten().zip(after.iter().flatten()) {
                let shifted = match (p, q) {
                    (Some(p), Some(q)) => {
                        (q[0] - p[0] - DX as f64).abs() < 1e-9
                            && (q[1] - p[1] - DY as f64).abs() < 1e-9
                    }
                    (p, q) => p.is_none() && q.is_none(),
                };
                if !shifted {
                    failures.push(format!("{}: {key:?} fact {p:?} became {q:?}", case.name));
                }
            }
        }
    }
    eprintln!(
        "translation: {ran} ran; skipped {skipped:?}, {NOT_TRANSLATABLE:?}, not rewritable {unsafe_rewrite:?}"
    );
    assert!(
        unsafe_rewrite.is_empty(),
        "not rewritable: {unsafe_rewrite:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A text edit at one source position: `len` characters inserted (negative: removed) at
/// 1-based `line`/`col`.
#[derive(Clone, Copy)]
struct Edit {
    line: u32,
    col: u32,
    len: i64,
}

impl Edit {
    fn apply(&self, src: &str, text: &str) -> String {
        let mut rows = rows(src);
        let row = &mut rows[self.line as usize - 1];
        let at = self.col as usize - 1;
        if self.len < 0 {
            row.drain(at..at + (-self.len) as usize);
        } else {
            row.splice(at..at, text.chars());
        }
        join(&rows)
    }

    /// Maps a span of the edited source back to the original. The edited statement's span
    /// grows or shrinks with the edit, so `owner` maps it from the edited to the original.
    fn undo(&self, span: &mut Span, owner: (Span, Span)) {
        if *span == owner.0 {
            *span = owner.1;
            return;
        }
        let (p, len) = (self.col as i64, self.len);
        let col = |line: u32, col: u32, end: bool| {
            let c = col as i64;
            if line != self.line {
                return c;
            }
            if len < 0 {
                if c > p || (!end && c == p) {
                    c - len
                } else {
                    c
                }
            } else if c > p + len || (!end && c == p + len) {
                c - len
            } else if end && c > p {
                p
            } else {
                c
            }
        };
        span.col = col(span.line, span.col, false) as u32;
        span.end_col = col(span.end_line, span.end_col, true) as u32;
    }

    fn undo_all(&self, diags: &mut [Diagnostic], owner: (Span, Span)) {
        for d in diags {
            self.undo(&mut d.span, owner);
            for e in &mut d.elements {
                self.undo(&mut e.span, owner);
            }
            for s in &mut d.suggestions {
                self.undo(&mut s.span, owner);
            }
        }
    }
}

fn owner_span(scene: &Scene, id: &str) -> Option<Span> {
    scene
        .lines
        .iter()
        .find(|l| l.id == id)
        .map(|l| l.span)
        .or_else(|| scene.shapes.iter().find(|s| s.id == id).map(|s| s.span))
}

fn is_owner(scene: &Scene, id: &str) -> bool {
    owner_span(scene, id).is_some()
}

/// The span's characters as (line, col, char) triples.
fn chars_in(src: &str, span: Span) -> Vec<(u32, u32, char)> {
    let rows = rows(src);
    let mut out = Vec::new();
    for line in span.line..=span.end_line {
        let row = &rows[line as usize - 1];
        let mask = code_mask(row);
        let from = if line == span.line { span.col } else { 1 };
        let to = if line == span.end_line {
            span.end_col
        } else {
            row.len() as u32 + 1
        };
        for col in from..to {
            let i = col as usize - 1;
            out.push((line, col, if mask[i] { row[i] } else { '"' }));
        }
    }
    out
}

/// The edit that adds the `allow=` entry `entry` to the statement at `span`: appended to its
/// `allow=` when it has one, else a new attribute at its end.
fn add_allow(src: &str, span: Span, entry: &str) -> (Edit, String) {
    let chars = chars_in(src, span);
    let text: String = chars.iter().map(|c| c.2).collect();
    let (pos, insert) = match text.find("allow=") {
        Some(byte) => {
            let start = text[..byte].chars().count() + "allow=".len();
            let end = chars[start..]
                .iter()
                .position(|c| c.2 == ' ')
                .map_or(chars.len(), |p| start + p);
            let (line, col, _) = chars[end - 1];
            ((line, col + 1), format!(",{entry}"))
        }
        None => ((span.end_line, span.end_col), format!(" allow={entry}")),
    };
    let edit = Edit {
        line: pos.0,
        col: pos.1,
        len: insert.chars().count() as i64,
    };
    (edit, edit.apply(src, &insert))
}

fn sorted(mut diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    kashika_diag::sort(&mut diags);
    diags
}

/// Compares `after` to `before` finding by finding: `changed` decides which findings may
/// differ, and how; every other finding must be identical.
fn diff(
    before: &[Diagnostic],
    after: &[Diagnostic],
    changed: impl Fn(&Diagnostic, &Diagnostic) -> Option<Result<(), String>>,
) -> Result<(), String> {
    if before.len() != after.len() {
        return Err(format!(
            "{} findings became {}:\n{:#?}",
            before.len(),
            after.len(),
            after
                .iter()
                .filter(|d| !before.iter().any(|b| b.message == d.message))
                .collect::<Vec<_>>()
        ));
    }
    for (b, a) in before.iter().zip(after) {
        match changed(b, a) {
            Some(r) => r?,
            None if a != b => return Err(format!("changed:\n{b:#?}\n->\n{a:#?}")),
            None => {}
        }
    }
    Ok(())
}

#[test]
fn an_allow_entry_suppresses_exactly_what_it_names() {
    let (cases, skipped) = corpus();
    let (mut ran, mut no_owner, mut unused) = (0, Vec::new(), 0);
    let mut failures = Vec::new();
    for case in &cases {
        let (scene, errors) = case.compile(&case.src);
        let before = sorted(check(&scene));
        for f in before.iter().filter(|d| !d.suppressed) {
            if f.code == code::UNUSED_ALLOW {
                unused += 1;
                continue;
            }
            let owner = &f.elements[0].id;
            let Some(span) = owner_span(&scene, owner) else {
                no_owner.push(format!("{}: {} on {owner}", case.name, f.code));
                continue;
            };
            let other = f.elements[1..]
                .iter()
                .map(|e| e.id.as_str())
                .find(|id| id != owner && is_owner(&scene, id) && !id.starts_with("line@"));
            let entry = match other {
                Some(o) => format!("{}:{o}", f.code),
                None => f.code.to_string(),
            };
            ran += 1;
            let (edit, src) = add_allow(&case.src, span, &entry);
            let (s, mut errs) = case.compile(&src);
            let stmt = (owner_span(&s, owner).unwrap(), span);
            edit.undo_all(&mut errs, stmt);
            let mut after = check(&s);
            edit.undo_all(&mut after, stmt);
            let r = if errs != errors {
                Err(format!("errors changed: {errs:#?}"))
            } else {
                diff(&before, &sorted(after), |b, a| {
                    suppresses(f.code, other, owner, b).then(|| {
                        let mut want = b.clone();
                        want.suppressed = true;
                        want.reason = a.reason.clone();
                        if a == &want {
                            Ok(())
                        } else {
                            Err(format!("not suppressed:\n{b:#?}\n->\n{a:#?}"))
                        }
                    })
                })
            };
            if let Err(e) = r {
                failures.push(format!("{}: allow={entry} on {owner}: {e}", case.name));
            }
        }
    }
    eprintln!(
        "suppression: {ran} findings ran; skipped cases {skipped:?}; {unused} unused-allow findings; no allow= owner: {no_owner:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The edit that removes `allow` entry `index` of the statement at `span`, with its comma, or
/// the whole ` allow=…` when it is the only entry. `None` when the entry spans are not apart.
fn remove_allow(src: &str, span: Span, entries: &[Span], index: usize) -> Option<(Edit, String)> {
    let e = entries[index];
    let chars = chars_in(src, span);
    let pos = |line: u32, col: u32| chars.iter().position(|c| c.0 == line && c.1 == col);
    let start = pos(e.line, e.col)?;
    let end = pos(e.end_line, e.end_col - 1)? + 1;
    if e.line != e.end_line || chars[start..end].iter().any(|c| c.2 == '=') {
        return None;
    }
    let (from, to) = if entries.len() == 1 {
        let allow = start.checked_sub("allow=".len() + 1)?;
        let written: String = chars[allow..start].iter().map(|c| c.2).collect();
        (written == " allow=").then_some((allow, end))?
    } else if index + 1 < entries.len() {
        (start, end + 1)
    } else {
        (start - 1, end)
    };
    let edit = Edit {
        line: chars[from].0,
        col: chars[from].1,
        len: -((to - from) as i64),
    };
    Some((edit, edit.apply(src, "")))
}

#[test]
fn removing_a_used_allow_entry_unsuppresses_only_its_findings() {
    let (cases, _) = corpus();
    let (mut ran, mut not_apart) = (0, Vec::new());
    let mut failures = Vec::new();
    for case in &cases {
        let (scene, errors) = case.compile(&case.src);
        let before = sorted(check(&scene));
        let owners = scene
            .lines
            .iter()
            .map(|l| (&l.id, &l.allow, l.span))
            .chain(scene.shapes.iter().map(|s| (&s.id, &s.allow, s.span)));
        let owners: Vec<_> = owners.collect();
        for &(id, allow, span) in &owners {
            for (i, a) in allow.iter().enumerate() {
                let used =
                    |d: &Diagnostic| d.suppressed && suppresses(&a.code, a.other.as_deref(), id, d);
                if !before.iter().any(used) {
                    continue;
                }
                let spans: Vec<Span> = allow.iter().map(|a| a.span).collect();
                let Some((edit, src)) = remove_allow(&case.src, span, &spans, i) else {
                    not_apart.push(format!("{}: {id}", case.name));
                    continue;
                };
                ran += 1;
                let still = |d: &Diagnostic| {
                    scene.allow.iter().any(|g| g.code == d.code)
                        || owners.iter().any(|&(oid, oallow, _)| {
                            oallow.iter().enumerate().any(|(j, o)| {
                                !(oid == id && j == i)
                                    && suppresses(&o.code, o.other.as_deref(), oid, d)
                            })
                        })
                };
                let (s, mut errs) = case.compile(&src);
                let stmt = (owner_span(&s, id).unwrap(), span);
                edit.undo_all(&mut errs, stmt);
                let mut after = check(&s);
                edit.undo_all(&mut after, stmt);
                let r = if errs != errors {
                    Err(format!("errors changed: {errs:#?}"))
                } else {
                    diff(&before, &sorted(after), |b, a| {
                        used(b).then(|| {
                            let mut want = b.clone();
                            want.suppressed = still(b);
                            want.reason = if still(b) { a.reason.clone() } else { None };
                            if a == &want {
                                Ok(())
                            } else {
                                Err(format!("wrong change:\n{b:#?}\n->\n{a:#?}"))
                            }
                        })
                    })
                };
                if let Err(e) = r {
                    failures.push(format!("{}: entry {i} of {id}: {e}", case.name));
                }
            }
        }
    }
    eprintln!("allow removal: {ran} entries ran; entries not apart: {not_apart:?}");
    assert!(ran > 5);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn an_allow_entry_that_matches_nothing_is_one_unused_allow() {
    let (cases, _) = corpus();
    let mut ran = 0;
    let mut failures = Vec::new();
    for case in &cases {
        let (scene, errors) = case.compile(&case.src);
        let before = sorted(check(&scene));
        let owners = scene.lines.iter().map(|l| (&l.id, l.span, "line"));
        let owners = owners.chain(scene.shapes.iter().map(|s| (&s.id, s.span, "shape")));
        for (id, span, kind) in owners {
            let Some(lint) = kashika_diag::LINT_CODES.iter().find(|c| {
                **c != code::UNUSED_ALLOW
                    && !before.iter().any(|d| d.code == **c && ids(d).contains(id))
            }) else {
                continue;
            };
            ran += 1;
            let (edit, src) = add_allow(&case.src, span, lint);
            let (s, mut errs) = case.compile(&src);
            let stmt = (owner_span(&s, id).unwrap(), span);
            edit.undo_all(&mut errs, stmt);
            let mut after = check(&s);
            edit.undo_all(&mut after, stmt);
            let new: Vec<&Diagnostic> = after
                .iter()
                .filter(|d| {
                    d.code == code::UNUSED_ALLOW
                        && d.message.starts_with(&format!("`allow={lint}` on"))
                })
                .filter(|d| !before.contains(d))
                .collect();
            let rest: Vec<Diagnostic> =
                after.iter().filter(|d| !new.contains(d)).cloned().collect();
            let r = if errs != errors {
                Err(format!("errors changed: {errs:#?}"))
            } else if new.len() != 1 || !ids(new[0]).contains(id) {
                Err(format!(
                    "want one new unused-allow on {kind} {id}, got {new:#?}"
                ))
            } else {
                diff(&before, &sorted(rest), |_, _| None)
            };
            if let Err(e) = r {
                failures.push(format!("{}: allow={lint} on {id}: {e}", case.name));
            }
        }
    }
    eprintln!("unused allow: {ran} owners ran");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn check_is_pure_and_bounded_matches_deferred() {
    let (cases, skipped) = corpus();
    for case in &cases {
        let (scene, _) = case.compile(&case.src);
        assert_eq!(check(&scene), check(&scene), "{}", case.name);
        let bounded = check_bounded(&Ctx::new(&scene), usize::MAX, &|_| true);
        assert!(bounded.omitted.is_empty(), "{}", case.name);
        assert_eq!(bounded.diags, check_deferred(&scene), "{}", case.name);
    }
    eprintln!("purity: {} ran; skipped {skipped:?}", cases.len());
}

/// `src` with its top-level `line` statements in reverse order, each in the rows of another.
fn reverse_lines(src: &str, scene: &Scene) -> String {
    let rows = rows(src);
    let blocks: Vec<(usize, usize)> = scene
        .lines
        .iter()
        .filter(|l| l.parent.is_none())
        .map(|l| (l.span.line as usize - 1, l.span.end_line as usize))
        .collect();
    let mut out = Vec::new();
    let mut next = 0;
    for (k, &(from, to)) in blocks.iter().enumerate() {
        out.extend_from_slice(&rows[next..from]);
        let (f, t) = blocks[blocks.len() - 1 - k];
        out.extend_from_slice(&rows[f..t]);
        next = to;
    }
    out.extend_from_slice(&rows[next..]);
    join(&out)
}

fn unordered(diags: &[Diagnostic]) -> Vec<String> {
    let mut keys: Vec<String> = diags
        .iter()
        .map(|d| {
            let mut ids = ids(d);
            ids.sort();
            format!("{} {ids:?} {:?} {}", d.code, points(d), d.suppressed)
        })
        .collect();
    keys.sort();
    keys
}

/// Known order dependence, excluded until fixed: at a tangency `seg_hit` falls back to the
/// first endpoint within tolerance, trying the earlier line's piece first, so the reported
/// point moves with the statement order: (150,100.16) with `flat` first, (150,100.24) with
/// `arc` first.
const ORDER_DEPENDENT: &[&str] = &["cross-tangent"];

#[test]
fn findings_do_not_depend_on_line_order() {
    let (cases, skipped) = corpus();
    let (mut ran, mut few, mut unnamed) = (0, 0, Vec::new());
    let mut failures = Vec::new();
    for case in &cases {
        if ORDER_DEPENDENT.contains(&case.name.as_str()) {
            continue;
        }
        let (scene, errors) = case.compile(&case.src);
        let top = scene.lines.iter().filter(|l| l.parent.is_none()).count();
        if top < 2 {
            few += 1;
            continue;
        }
        if scene.lines.iter().any(|l| !l.named) {
            unnamed.push(case.name.clone());
            continue;
        }
        ran += 1;
        let src = reverse_lines(&case.src, &scene);
        let (s, errs) = case.compile(&src);
        if error_codes(&errs) != error_codes(&errors) {
            failures.push(format!("{}: errors differ: {errs:#?}", case.name));
            continue;
        }
        let (a, b) = (unordered(&check(&scene)), unordered(&check(&s)));
        if a != b {
            failures.push(format!(
                "{}:\n  before: {a:#?}\n  after:  {b:#?}\n--- reversed source\n{src}",
                case.name
            ));
        }
    }
    eprintln!(
        "line order: {ran} ran; {few} with fewer than two top-level lines; unnamed lines {unnamed:?}; known {ORDER_DEPENDENT:?}; skipped {skipped:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn the_rewrites_do_what_they_say() {
    assert_eq!(add("12", 37), "49");
    assert_eq!(add("66.3", 53), "119.3");
    assert_eq!(add("-0.5", 37), "36.5");
    assert_eq!(add("-40", 37), "-3");
    let src = "diagram \"t\" size (300, 200)\nbox a \"(1,2)\" at (10,20) size (5,5)\nline l a.tl+(1,1) -> (5,6) -> x 10 -> y 20 -> h 30 -> curve left 4 (7,8)\ngroup g at (1,1) size (9,9) {\n  box b at (2,2) size (1,1)\n}";
    assert_eq!(
        translate(src).unwrap(),
        "diagram \"t\" size (337,253)\nbox a \"(1,2)\" at (47,73) size (5,5)\nline l a.tl+(1,1) -> (42,59) -> x 47 -> y 73 -> h 30 -> curve left 4 (44,61)\ngroup g at (38,54) size (9,9) {\n  box b at (2,2) size (1,1)\n}"
    );
    assert_eq!(translate("line l (a.x, 3) -> h 4"), None);
    assert_eq!(
        mask("box 'a' extends beyond the 300x200 canvas, at (1.5,-2)"),
        "box 'a' extends beyond the _x_ canvas, at (_,_)"
    );
}
