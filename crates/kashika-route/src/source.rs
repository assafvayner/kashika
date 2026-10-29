use kashika_diag::Span;

/// The source as the lexer counts it: lines of chars, without a leading BOM.
pub(crate) struct Source<'a> {
    src: &'a str,
    lines: Vec<Vec<char>>,
}

impl<'a> Source<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Source {
            src,
            lines: src
                .strip_prefix('\u{feff}')
                .unwrap_or(src)
                .split('\n')
                .map(|l| l.chars().collect())
                .collect(),
        }
    }

    /// Source text of a single-line span.
    pub(crate) fn slice(&self, span: Span) -> String {
        let line = &self.lines[span.line as usize - 1];
        let end = if span.end_line == span.line {
            span.end_col as usize - 1
        } else {
            line.len()
        };
        line[span.col as usize - 1..end].iter().collect()
    }

    /// The source with `stmts` on lines of their own right after `after` (1-based line and
    /// exclusive column), or at the end when `None`, and the line the first one lands on.
    pub(crate) fn with_lines(&self, after: Option<(u32, u32)>, stmts: &[String]) -> (String, u32) {
        let block = stmts.join("\n");
        let Some((line, col)) = after else {
            return (
                format!("{}\n{block}\n", self.src),
                self.lines.len() as u32 + 1,
            );
        };
        let mut out = String::new();
        for (i, l) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            if i + 1 == line as usize {
                let at = (col as usize - 1).min(l.len());
                out.extend(&l[..at]);
                out.push('\n');
                out.push_str(&block);
                out.push('\n');
                out.extend(&l[at..]);
            } else {
                out.extend(l);
            }
        }
        (out, line + 1)
    }
}
