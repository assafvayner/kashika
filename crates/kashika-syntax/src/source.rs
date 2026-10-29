use kashika_diag::Span;

/// The source as the lexer counts it: lines of chars, without a leading byte order mark.
#[derive(Debug, Clone)]
pub struct SourceText {
    lines: Vec<Vec<char>>,
}

impl SourceText {
    pub fn new(src: &str) -> Self {
        Self::stripped(src.strip_prefix('\u{feff}').unwrap_or(src))
    }

    /// The text of `src`, which has no byte order mark.
    pub(crate) fn stripped(src: &str) -> Self {
        SourceText {
            lines: src.split('\n').map(|l| l.chars().collect()).collect(),
        }
    }

    pub fn lines(&self) -> &[Vec<char>] {
        &self.lines
    }

    /// The text of `span`.
    pub fn span(&self, span: Span) -> String {
        self.between(span, span)
    }

    /// The text from the start of `from` to the end of `to`, its lines joined by a space.
    /// Whatever lies outside the source is left out.
    pub fn between(&self, from: Span, to: Span) -> String {
        let mut out = String::new();
        for line in from.line..=to.end_line {
            let Some(chars) = (line as usize)
                .checked_sub(1)
                .and_then(|i| self.lines.get(i))
            else {
                break;
            };
            let a = if line == from.line {
                (from.col as usize).saturating_sub(1)
            } else {
                0
            };
            let b = if line == to.end_line {
                (to.end_col as usize).saturating_sub(1)
            } else {
                chars.len()
            };
            if !out.is_empty() {
                out.push(' ');
            }
            out.extend(chars.get(a..b.min(chars.len())).unwrap_or_default());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_spans_by_char() {
        let s = SourceText::new("\u{feff}box a \"é\"\nline x: a -> b\n");
        assert_eq!(s.span(Span::new(1, 7, 1, 10)), "\"é\"");
        assert_eq!(s.span(Span::new(2, 1, 2, 5)), "line");
        assert_eq!(
            s.between(Span::new(1, 5, 1, 6), Span::new(2, 1, 2, 5)),
            "a \"é\" line"
        );
    }

    #[test]
    fn leaves_out_what_lies_outside_the_source() {
        let s = SourceText::new("box a\n");
        assert_eq!(s.span(Span::new(1, 5, 1, 99)), "a");
        assert_eq!(s.span(Span::new(9, 1, 9, 4)), "");
        assert_eq!(s.span(Span::new(1, 9, 1, 12)), "");
        assert_eq!(s.span(Span::default()), "");
    }
}
