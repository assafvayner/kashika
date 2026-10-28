use serde::Serialize;

pub const LINT_CODES: &[&str] = &[
    "cross",
    "shared-segment",
    "through",
    "near",
    "overlap",
    "spill",
    "diagonal-tail",
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
        }
    }

    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, span, message.into())
    }

    pub fn warn(code: &'static str, span: Span, message: impl Into<String>) -> Self {
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
        let v = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        self.facts.insert(key.to_string(), v);
        self
    }
}

pub fn did_you_mean<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|c| (edit_distance(name, c), c))
        .filter(|(d, c)| *d <= (c.chars().count().max(name.chars().count()) / 2).max(1))
        .min_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)))
        .map(|(_, c)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

pub fn sort(diags: &mut [Diagnostic]) {
    diags.sort_by(|a, b| {
        (a.span.line, a.span.col, a.severity, a.code).cmp(&(
            b.span.line,
            b.span.col,
            b.severity,
            b.code,
        ))
    });
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
    fn does_not_suggest_distant_names() {
        assert_eq!(did_you_mean("zzzzz", ["right"]), None);
    }

    #[test]
    fn sorts_by_position_then_severity() {
        let mut d = vec![
            Diagnostic::error("b", Span::new(3, 1, 3, 2), "x"),
            Diagnostic::error("a", Span::new(1, 5, 1, 6), "y"),
        ];
        sort(&mut d);
        assert_eq!(d[0].code, "a");
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
