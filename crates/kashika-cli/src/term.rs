//! Escaping for text that reaches the terminal, so input can't smuggle escape sequences.

use std::borrow::Cow;

/// `s` with control characters, DEL and bidirectional overrides written as `\u{..}`. Tabs
/// are kept, and so are newlines when `newlines` is set.
fn escape_with(s: &str, newlines: bool) -> Cow<'_, str> {
    let bad = |c: char| unsafe_char(c) && !(c == '\t' || (newlines && c == '\n'));
    if !s.chars().any(bad) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if bad(c) {
            out.push_str(&format!("\\u{{{:x}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

fn unsafe_char(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// One line of terminal output: every control character but tab is escaped.
pub fn line(s: &str) -> Cow<'_, str> {
    escape_with(s, false)
}

/// Multi-line terminal output: like `line`, but newlines are kept.
pub fn text(s: &str) -> Cow<'_, str> {
    escape_with(s, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_controls_and_keeps_tabs() {
        assert_eq!(line("a\u{1b}]52;c;x\u{7}b\tc"), "a\\u{1b}]52;c;x\\u{7}b\tc");
        assert_eq!(line("\u{7f}\u{9b}\n\r"), "\\u{7f}\\u{9b}\\u{a}\\u{d}");
        assert_eq!(line("\u{202e}abc"), "\\u{202e}abc");
        assert_eq!(text("a\nb\u{1b}"), "a\nb\\u{1b}");
        assert!(matches!(line("plain — ünïcode"), Cow::Borrowed(_)));
    }
}
