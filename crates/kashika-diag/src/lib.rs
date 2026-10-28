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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub help: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
            help: Vec::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help.push(help.into());
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
}
