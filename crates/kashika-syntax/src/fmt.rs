use kashika_diag::{Diagnostic, Span};

use crate::lexer::{Tok, Token, lex, lex_with_comments};

/// Rewrites `src` in canonical layout. Only whitespace changes: indentation, the spacing
/// between tokens, blank lines and trailing whitespace. Line breaks, comments and the
/// spelling of numbers, strings and colors are kept. Input with `parse` errors is refused.
pub fn format(src: &str) -> Result<String, Vec<Diagnostic>> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let (_, diags) = crate::parse(src);
    let errors: Vec<Diagnostic> = diags.into_iter().filter(|d| d.code == "parse").collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    let (toks, comments, _) = lex_with_comments(src);
    let lines: Vec<Vec<char>> = src.split('\n').map(|l| l.chars().collect()).collect();
    let mut items: Vec<Item> = toks
        .iter()
        .filter(|t| !matches!(t.tok, Tok::Newline | Tok::Eof))
        .map(|t| Item {
            text: tok_text(t, &lines),
            span: t.span,
            tok: Some(t.tok.clone()),
            line_comment: false,
        })
        .collect();
    items.extend(comments.into_iter().map(|c| Item {
        line_comment: c.text.starts_with("//"),
        text: c.text,
        span: c.span,
        tok: None,
    }));
    items.sort_by_key(|i| (i.span.line, i.span.col));
    let out = print(&items);
    if shape(&lex(&out).0) != shape(&toks) {
        return Err(vec![Diagnostic::error(
            "parse",
            Span::default(),
            "internal error: formatting would change the tokens; please report this file",
        )]);
    }
    Ok(out)
}

struct Item {
    text: String,
    span: Span,
    tok: Option<Tok>,
    line_comment: bool,
}

impl Item {
    fn is(&self, t: &Tok) -> bool {
        self.tok.as_ref() == Some(t)
    }

    fn is_ident(&self, s: &str) -> bool {
        matches!(&self.tok, Some(Tok::Ident(i)) if i == s)
    }
}

fn tok_text(t: &Token, lines: &[Vec<char>]) -> String {
    match &t.tok {
        Tok::Ident(s) => s.clone(),
        Tok::Num(_) | Tok::Str(_) | Tok::Color(_) => {
            let line = &lines[t.span.line as usize - 1];
            line[t.span.col as usize - 1..t.span.end_col as usize - 1]
                .iter()
                .collect()
        }
        Tok::LParen => "(".into(),
        Tok::RParen => ")".into(),
        Tok::LBrace => "{".into(),
        Tok::RBrace => "}".into(),
        Tok::Comma => ",".into(),
        Tok::Eq => "=".into(),
        Tok::Arrow => "->".into(),
        Tok::Dot => ".".into(),
        Tok::Plus => "+".into(),
        Tok::Minus => "-".into(),
        Tok::Colon => ":".into(),
        Tok::At => "@".into(),
        Tok::Newline | Tok::Eof => String::new(),
    }
}

/// Token kinds with runs of newlines collapsed and leading and trailing newlines dropped.
fn shape(toks: &[Token]) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    for t in toks {
        if t.tok == Tok::Newline && matches!(out.last(), None | Some(Tok::Newline)) {
            continue;
        }
        out.push(t.tok.clone());
    }
    if let [.., Tok::Newline, Tok::Eof] = out.as_slice() {
        let n = out.len();
        out.remove(n - 2);
    }
    out
}

/// Groups items by source line; an item that starts on a line a previous item of the row
/// (such as a multi-line block comment) ends on joins that row.
fn rows(items: &[Item]) -> Vec<&[Item]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut end_line = 0;
    for (i, it) in items.iter().enumerate() {
        if i > start && it.span.line > end_line {
            out.push(&items[start..i]);
            start = i;
            end_line = 0;
        }
        end_line = end_line.max(it.span.end_line);
    }
    if start < items.len() {
        out.push(&items[start..]);
    }
    out
}

fn print(items: &[Item]) -> String {
    let mut out = String::new();
    // One entry per open `{`; `true` when it opened a `style` block, whose lines are attributes.
    let mut blocks: Vec<bool> = Vec::new();
    let mut prev_end = 0u32;
    let mut prev_opened = false;
    for row in rows(items) {
        let first = &row[0];
        let closes = first.is(&Tok::RBrace);
        let depth = blocks.len().saturating_sub(usize::from(closes));
        let in_style = blocks.last().copied().unwrap_or(false);
        if prev_end > 0 && first.span.line > prev_end + 1 && !prev_opened && !closes {
            out.push('\n');
        }
        let cont = !in_style && continues(row);
        out.push_str(&"  ".repeat(depth + usize::from(cont)));
        out.push_str(&join(row));
        out.push('\n');
        let style = first.is_ident("style");
        for it in row {
            if it.is(&Tok::LBrace) {
                blocks.push(style);
            } else if it.is(&Tok::RBrace) {
                blocks.pop();
            }
        }
        prev_opened = row.last().is_some_and(|i| i.is(&Tok::LBrace));
        prev_end = row
            .iter()
            .map(|i| i.span.end_line)
            .max()
            .unwrap_or(prev_end);
    }
    out
}

/// A physical line the parser joins onto the statement above it.
fn continues(row: &[Item]) -> bool {
    match &row[0].tok {
        Some(Tok::Arrow) => true,
        Some(Tok::Ident(s)) => {
            s == "ortho"
                || matches!(
                    row.get(1).and_then(|i| i.tok.as_ref()),
                    Some(Tok::Eq | Tok::At)
                )
        }
        _ => false,
    }
}

fn join(row: &[Item]) -> String {
    let mut s = String::new();
    for (i, it) in row.iter().enumerate() {
        if i > 0 {
            if it.line_comment {
                s.push_str("  ");
            } else if space(row, i) {
                s.push(' ');
            }
        }
        s.push_str(&it.text);
    }
    s
}

fn space(row: &[Item], i: usize) -> bool {
    let (Some(p), Some(n)) = (&row[i - 1].tok, &row[i].tok) else {
        return true;
    };
    if matches!(
        n,
        Tok::RParen
            | Tok::Comma
            | Tok::Dot
            | Tok::At
            | Tok::Eq
            | Tok::Colon
            | Tok::Plus
            | Tok::Minus
    ) {
        return false;
    }
    match p {
        Tok::Comma => {
            let before = &row[..i];
            let open = before.iter().filter(|t| t.is(&Tok::LParen)).count();
            let close = before.iter().filter(|t| t.is(&Tok::RParen)).count();
            let ident = |t: Option<&Tok>| matches!(t, Some(Tok::Ident(_)));
            open > close && ((i >= 2 && ident(row[i - 2].tok.as_ref())) || ident(Some(n)))
        }
        Tok::LParen | Tok::Dot | Tok::At | Tok::Eq | Tok::Plus | Tok::Minus | Tok::Colon => false,
        _ => true,
    }
}
