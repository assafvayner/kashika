//! String escapes for plain-text fields (titles, reasons). The lexer keeps `\\`, `\*` and
//! `` \` `` in string values.

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
