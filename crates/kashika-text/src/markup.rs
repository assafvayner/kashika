use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    Regular,
    Bold,
    Mono,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    pub text: String,
    pub font: Font,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupError {
    pub marker: &'static str,
}

impl std::fmt::Display for MarkupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unclosed `{}` in text", self.marker)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Plain,
    Bold,
    Code,
}

/// Width of a tab, in spaces. A tab has no glyph in the bundled fonts, so `parse_markup`
/// replaces each one with this many spaces.
pub const TAB_WIDTH: usize = 4;

/// Splits `s` into lines of styled runs. `**bold**` and `` `code` `` don't nest: inside one,
/// the other marker is literal text. `\*` and `` \` `` are literal markers.
/// A tab becomes `TAB_WIDTH` spaces; there are no tab stops.
pub fn parse_markup(s: &str, bold: bool) -> Result<Vec<Vec<Run>>, MarkupError> {
    let plain = if bold { Font::Bold } else { Font::Regular };
    let font = |m: Mode| match m {
        Mode::Plain => plain,
        Mode::Bold => Font::Bold,
        Mode::Code => Font::Mono,
    };
    let mut lines: Vec<Vec<Run>> = vec![Vec::new()];
    let mut buf = String::new();
    let mut mode = Mode::Plain;
    let flush = |lines: &mut Vec<Vec<Run>>, buf: &mut String, f: Font| {
        if buf.is_empty() {
            return;
        }
        let line = lines.last_mut().expect("at least one line");
        match line.last_mut() {
            Some(r) if r.font == f => r.text.push_str(buf),
            _ => line.push(Run {
                text: buf.clone(),
                font: f,
            }),
        }
        buf.clear();
    };
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\\' if matches!(next, Some('*' | '`' | '\\')) => {
                buf.push(next.unwrap_or_default());
                i += 2;
                continue;
            }
            '\t' => buf.push_str(&" ".repeat(TAB_WIDTH)),
            '\n' => {
                flush(&mut lines, &mut buf, font(mode));
                lines.push(Vec::new());
            }
            '*' if next == Some('*') && mode != Mode::Code => {
                flush(&mut lines, &mut buf, font(mode));
                mode = if mode == Mode::Bold {
                    Mode::Plain
                } else {
                    Mode::Bold
                };
                i += 2;
                continue;
            }
            '`' if mode != Mode::Bold => {
                flush(&mut lines, &mut buf, font(mode));
                mode = if mode == Mode::Code {
                    Mode::Plain
                } else {
                    Mode::Code
                };
            }
            _ => buf.push(c),
        }
        i += 1;
    }
    match mode {
        Mode::Plain => {
            flush(&mut lines, &mut buf, font(mode));
            Ok(lines)
        }
        Mode::Bold => Err(MarkupError { marker: "**" }),
        Mode::Code => Err(MarkupError { marker: "`" }),
    }
}
