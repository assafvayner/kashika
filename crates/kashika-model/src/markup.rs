//! Inline markup (spec §6). The lexer keeps the escapes `\\`, `\*` and `` \` `` in
//! string values; this layer detects markers and turns the escapes into literal characters.

/// Whether `s` contains an unescaped `**` or `` ` `` marker.
pub fn has_markup(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '`' => return true,
            '*' if chars.peek() == Some(&'*') => return true,
            _ => {}
        }
    }
    false
}

/// Replaces the escapes `\\`, `\*` and `` \` `` with the literal characters.
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
            continue;
        }
        out.push(c);
    }
    out
}
