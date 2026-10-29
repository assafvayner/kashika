use kashika_diag::Span;
use kashika_syntax::SourceText;

/// The source, and its text as the lexer counts it.
pub(crate) struct Source<'a> {
    src: &'a str,
    text: SourceText,
}

impl<'a> Source<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Source {
            src,
            text: SourceText::new(src),
        }
    }

    pub(crate) fn slice(&self, span: Span) -> String {
        self.text.span(span)
    }

    /// The source with `stmts` on lines of their own right after `after` (1-based line and
    /// exclusive column), or at the end when `None`, and the line the first one lands on.
    pub(crate) fn with_lines(&self, after: Option<(u32, u32)>, stmts: &[String]) -> (String, u32) {
        let block = stmts.join("\n");
        let Some((line, col)) = after else {
            return (
                format!("{}\n{block}\n", self.src),
                self.text.lines().len() as u32 + 1,
            );
        };
        let mut out = String::new();
        for (i, l) in self.text.lines().iter().enumerate() {
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
