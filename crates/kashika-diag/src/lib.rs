use std::any::Any;
use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use serde::Serialize;

/// Every error code, in the order of the spec's §7 Errors table.
pub const ERROR_CODES: &[&str] = &[
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
    "unknown-shape",
    "size",
    "range",
    "diagram",
    "version",
    "file",
];

pub const LINT_CODES: &[&str] = &[
    "cross",
    "shared-segment",
    "through",
    "near",
    "overlap",
    "spill",
    "diagonal-tail",
    "near-axis",
    "jog",
    "endpoint-off-edge",
    "port-crowding",
    "group-transit",
    "low-contrast",
    "label-collision",
    "out-of-canvas",
    "off-grid",
    "block-reducible",
    "text-outside-shape",
    "image-distort",
    "svg-dropped",
    "missing-glyph",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn new(line: u32, col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            line,
            col,
            end_line,
            end_col,
        }
    }

    pub fn to(self, other: Span) -> Span {
        Span {
            line: self.line,
            col: self.col,
            end_line: other.end_line,
            end_col: other.end_col,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
}

/// An element a diagnostic is about, with enough geometry to locate it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Element {
    pub id: String,
    /// `box`, `group`, `line`, `junction`, `text` or `label`.
    pub kind: &'static str,
    pub span: Span,
    /// 1-based index of the authored `-> ...` segment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[[f64; 2]; 2]>,
    /// Id of the group whose frame the element's literals use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_bbox: Option<[[f64; 2]; 2]>,
}

impl Element {
    pub fn new(id: impl Into<String>, kind: &'static str, span: Span) -> Self {
        Self {
            id: id.into(),
            kind,
            span,
            segment: None,
            from: None,
            to: None,
            bbox: None,
            frame: None,
            local_bbox: None,
        }
    }
}

/// A source edit the author may paste. Never applied automatically.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Suggestion {
    pub span: Span,
    pub replacement: String,
    /// Ids whose geometry would change.
    pub ripple: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<Element>,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub facts: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub help: Vec<String>,
    pub suggestions: Vec<Suggestion>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub suppressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Work a later pass still has to do, such as the costly help `kashika_check::resolve`
    /// fills in only for the diagnostics that are shown.
    #[serde(skip)]
    pub pending: Pending,
}

/// Opaque work attached to a diagnostic by the pass that made it and taken back by a later
/// one. It takes no part in equality or serialization.
#[derive(Clone, Default)]
pub struct Pending(Option<Arc<dyn Any + Send + Sync>>);

impl Pending {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Some(Arc::new(value)))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.as_deref()?.downcast_ref()
    }

    /// Takes the value out when it is a `T`, leaving the slot empty; otherwise changes nothing.
    pub fn take<T: Any + Send + Sync + Clone>(&mut self) -> Option<T> {
        self.get::<T>()?;
        let v = self.0.take()?.downcast::<T>().ok()?;
        Some(Arc::unwrap_or_clone(v))
    }
}

impl PartialEq for Pending {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_empty() {
            "Pending"
        } else {
            "Pending(..)"
        })
    }
}

impl Diagnostic {
    fn new(severity: Severity, code: &'static str, span: Span, message: String) -> Self {
        Self {
            severity,
            code,
            message,
            span,
            elements: Vec::new(),
            facts: serde_json::Map::new(),
            help: Vec::new(),
            suggestions: Vec::new(),
            suppressed: false,
            reason: None,
            pending: Pending::default(),
        }
    }

    /// An error; `code` must be in `ERROR_CODES`.
    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        debug_assert!(ERROR_CODES.contains(&code), "`{code}` is not an error code");
        Self::new(Severity::Error, code, span, message.into())
    }

    /// A warning; `code` must be in `LINT_CODES`.
    pub fn warn(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        debug_assert!(LINT_CODES.contains(&code), "`{code}` is not a lint code");
        Self::new(Severity::Warn, code, span, message.into())
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help.push(help.into());
        self
    }

    pub fn with_element(mut self, element: Element) -> Self {
        self.elements.push(element);
        self
    }

    pub fn with_fact(mut self, key: &str, value: impl Serialize) -> Self {
        let v = serde_json::to_value(value).expect("a fact serializes to JSON");
        self.facts.insert(key.to_string(), v);
        self
    }
}

/// The candidate closest to `name` that is at least one edit away (an insertion, deletion,
/// substitution or swap of neighbors), at most a third of the characters of the longer of
/// the two away (counting at least 3, so one edit up to five characters, two up to eight),
/// and fewer edits away than it has characters, so a one-letter candidate is never suggested.
pub fn did_you_mean<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|c| (edit_distance(name, c), c))
        .filter(|&(d, c)| {
            let len = c.chars().count();
            d > 0 && d < len && d <= len.max(name.chars().count()).max(3) / 3
        })
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)))
        .map(|(_, c)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut older: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                cur[j] = cur[j].min(older[j - 2] + 1);
            }
        }
        older = std::mem::replace(&mut prev, cur);
    }
    prev[b.len()]
}

/// Sorts into `order`. The sort is stable, so diagnostics that `order` ties keep their order.
pub fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(order);
}

/// Source order: by start, then severity, code, end and message.
pub fn order(a: &Diagnostic, b: &Diagnostic) -> Ordering {
    let key = |d: &Diagnostic| {
        (
            d.span.line,
            d.span.col,
            d.severity,
            d.code,
            d.span.end_line,
            d.span.end_col,
        )
    };
    key(a).cmp(&key(b)).then_with(|| a.message.cmp(&b.message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_names() {
        assert_eq!(did_you_mean("rigth", ["right", "left"]), Some("right"));
        assert_eq!(did_you_mean("bx", ["box", "text", "line"]), Some("box"));
    }

    #[test]
    fn suggests_real_typos() {
        assert_eq!(
            did_you_mean("boundry", ["boundary", "box"]),
            Some("boundary")
        );
        assert_eq!(did_you_mean("pinn", ["pin"]), Some("pin"));
        assert_eq!(did_you_mean("colr", ["color"]), Some("color"));
        assert_eq!(
            did_you_mean("stroke-widht", ["stroke-width"]),
            Some("stroke-width")
        );
    }

    #[test]
    fn does_not_suggest_loose_matches() {
        assert_eq!(did_you_mean("icon", ["cdn"]), None);
        assert_eq!(did_you_mean("condition", ["function"]), None);
        assert_eq!(did_you_mean("centre", ["center"]), Some("center"));
        assert_eq!(did_you_mean("abcdef", ["abcxyz"]), None);
    }

    #[test]
    fn does_not_suggest_distant_names() {
        assert_eq!(did_you_mean("zzzzz", ["right"]), None);
    }

    #[test]
    fn never_suggests_the_name_itself() {
        assert_eq!(did_you_mean("right", ["right"]), None);
        assert_eq!(did_you_mean("right", ["right", "rigt"]), Some("rigt"));
    }

    #[test]
    fn never_suggests_one_letter_names() {
        assert_eq!(did_you_mean("c", ["a", "b"]), None);
        assert_eq!(did_you_mean("ab", ["a"]), None);
        assert_eq!(did_you_mean("x", ["xy"]), Some("xy"));
    }

    #[test]
    fn suggests_nothing_without_candidates() {
        assert_eq!(did_you_mean("box", []), None);
        assert_eq!(did_you_mean("", ["a", "ab"]), None);
    }

    #[test]
    fn ties_go_to_the_alphabetically_first_candidate() {
        assert_eq!(did_you_mean("cat", ["cot", "bat"]), Some("bat"));
    }

    #[test]
    fn span_to_joins_start_and_end() {
        let a = Span::new(2, 3, 2, 9);
        let b = Span::new(4, 1, 5, 7);
        assert_eq!(a.to(b), Span::new(2, 3, 5, 7));
        assert_eq!(a.to(a), a);
    }

    #[test]
    fn sorts_by_position_then_severity() {
        let mut d = vec![
            Diagnostic::error("size", Span::new(3, 1, 3, 2), "x"),
            Diagnostic::error("attr", Span::new(1, 5, 1, 6), "y"),
        ];
        sort(&mut d);
        assert_eq!(d[0].code, "attr");
    }

    #[test]
    fn sorts_errors_before_warnings_then_by_code_end_and_message() {
        let at = Span::new(1, 1, 1, 5);
        let mut d = vec![
            Diagnostic::warn("near", at, "b"),
            Diagnostic::warn("near", at, "a"),
            Diagnostic::warn("near", Span::new(1, 1, 1, 3), "c"),
            Diagnostic::warn("cross", at, "z"),
            Diagnostic::error("parse", at, "e"),
        ];
        sort(&mut d);
        let got: Vec<(&str, &str)> = d.iter().map(|d| (d.code, d.message.as_str())).collect();
        assert_eq!(
            got,
            [
                ("parse", "e"),
                ("cross", "z"),
                ("near", "c"),
                ("near", "a"),
                ("near", "b")
            ]
        );
    }

    #[test]
    fn pending_work_is_ignored_by_equality() {
        let mut a = Diagnostic::warn("near", Span::default(), "x");
        let b = a.clone();
        a.pending = Pending::new(vec![1u8]);
        assert_eq!(a, b);
        assert_eq!(a.pending.take::<String>(), None);
        assert_eq!(a.pending.take::<Vec<u8>>(), Some(vec![1]));
        assert!(a.pending.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "not a lint code")]
    fn warnings_need_a_lint_code() {
        let _ = Diagnostic::warn("parse", Span::default(), "x");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "not an error code")]
    fn errors_need_an_error_code() {
        let _ = Diagnostic::error("cross", Span::default(), "x");
    }

    #[test]
    fn warnings_serialize_elements_facts_and_suppression() {
        let span = Span::new(2, 1, 2, 9);
        let mut d = Diagnostic::warn("cross", span, "a crosses b")
            .with_element(Element {
                segment: Some(1),
                from: Some([0.0, 0.0]),
                to: Some([10.0, 0.0]),
                ..Element::new("a", "line", span)
            })
            .with_fact("at", [5.0, 0.0]);
        d.suppressed = true;
        d.reason = Some("bus".into());
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["severity"], "warn");
        assert_eq!(v["elements"][0]["id"], "a");
        assert_eq!(v["elements"][0]["segment"], 1);
        assert!(v["elements"][0].get("bbox").is_none());
        assert_eq!(v["facts"]["at"], serde_json::json!([5.0, 0.0]));
        assert_eq!(v["suggestions"], serde_json::json!([]));
        assert_eq!(v["suppressed"], true);
        assert_eq!(v["reason"], "bus");

        let e = serde_json::to_value(Diagnostic::error("parse", Span::default(), "x")).unwrap();
        assert!(e.get("elements").is_none());
        assert!(e.get("facts").is_none());
        assert!(e.get("suppressed").is_none());
        assert!(e.get("reason").is_none());
    }
}
