use kashika_diag::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Num(f64),
    Str(String),
    Color(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Eq,
    Arrow,
    Dot,
    Plus,
    Colon,
    At,
    Newline,
    Eof,
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Tok::Ident(s) => format!("`{s}`"),
            Tok::Num(n) => format!("number `{n}`"),
            Tok::Str(_) => "a string".into(),
            Tok::Color(c) => format!("color `{c}`"),
            Tok::LParen => "`(`".into(),
            Tok::RParen => "`)`".into(),
            Tok::LBrace => "`{`".into(),
            Tok::RBrace => "`}`".into(),
            Tok::Comma => "`,`".into(),
            Tok::Eq => "`=`".into(),
            Tok::Arrow => "`->`".into(),
            Tok::Dot => "`.`".into(),
            Tok::Plus => "`+`".into(),
            Tok::Colon => "`:`".into(),
            Tok::At => "`@`".into(),
            Tok::Newline => "end of line".into(),
            Tok::Eof => "end of file".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer {
        chars: src.chars().collect(),
        pos: 0,
        line: 1,
        col: 1,
        toks: Vec::new(),
        diags: Vec::new(),
    };
    lx.run();
    (lx.toks, lx.diags)
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    toks: Vec<Token>,
    diags: Vec<Diagnostic>,
}

impl Lexer {
    fn peek(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn push(&mut self, tok: Tok, line: u32, col: u32) {
        self.toks.push(Token {
            tok,
            span: Span::new(line, col, self.line, self.col),
        });
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(0) {
            let (line, col) = (self.line, self.col);
            match c {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    self.toks.push(Token {
                        tok: Tok::Newline,
                        span: Span::new(line, col, line, col + 1),
                    });
                }
                '/' if self.peek(1) == Some('/') => {
                    while self.peek(0).is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                '"' => self.string(line, col),
                '#' => self.color(line, col),
                '-' if self.peek(1) == Some('>') => {
                    self.bump();
                    self.bump();
                    self.push(Tok::Arrow, line, col);
                }
                '-' if self.peek(1).is_some_and(|c| c.is_ascii_digit()) => self.number(line, col),
                c if c.is_ascii_digit() => self.number(line, col),
                c if c.is_ascii_alphabetic() || c == '_' => self.ident(line, col),
                '(' | ')' | '{' | '}' | ',' | '=' | '.' | '+' | ':' | '@' => {
                    self.bump();
                    let tok = match c {
                        '(' => Tok::LParen,
                        ')' => Tok::RParen,
                        '{' => Tok::LBrace,
                        '}' => Tok::RBrace,
                        ',' => Tok::Comma,
                        '=' => Tok::Eq,
                        '.' => Tok::Dot,
                        '+' => Tok::Plus,
                        ':' => Tok::Colon,
                        _ => Tok::At,
                    };
                    self.push(tok, line, col);
                }
                other => {
                    self.bump();
                    self.diags.push(Diagnostic::error(
                        "parse",
                        Span::new(line, col, line, col + 1),
                        format!("unexpected character `{other}`"),
                    ));
                }
            }
        }
        let (line, col) = (self.line, self.col);
        self.toks.push(Token {
            tok: Tok::Eof,
            span: Span::new(line, col, line, col),
        });
    }

    fn number(&mut self, line: u32, col: u32) {
        let mut s = String::new();
        if self.peek(0) == Some('-') {
            s.push('-');
            self.bump();
        }
        while let Some(c) = self.peek(0).filter(char::is_ascii_digit) {
            s.push(c);
            self.bump();
        }
        if self.peek(0) == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            s.push('.');
            self.bump();
            while let Some(c) = self.peek(0).filter(char::is_ascii_digit) {
                s.push(c);
                self.bump();
            }
        }
        let n = s
            .parse::<f64>()
            .expect("lexer only collects valid number text");
        self.push(Tok::Num(n), line, col);
    }

    fn ident(&mut self, line: u32, col: u32) {
        let mut s = String::new();
        while let Some(c) = self.peek(0) {
            let continues = c.is_ascii_alphanumeric()
                || c == '_'
                || (c == '-'
                    && self
                        .peek(1)
                        .is_some_and(|n| n.is_ascii_alphanumeric() || n == '_'));
            if !continues {
                break;
            }
            s.push(c);
            self.bump();
        }
        self.push(Tok::Ident(s), line, col);
    }

    fn string(&mut self, line: u32, col: u32) {
        self.bump();
        let mut s = String::new();
        loop {
            match self.peek(0) {
                None | Some('\n') => {
                    self.diags.push(
                        Diagnostic::error(
                            "parse",
                            Span::new(line, col, self.line, self.col),
                            "unterminated string",
                        )
                        .with_help("close the string with `\"` before the end of the line"),
                    );
                    break;
                }
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    self.bump();
                    match self.bump() {
                        Some('n') => s.push('\n'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        // Markup escapes like `\*` are kept for the text layer.
                        Some(other) => {
                            s.push('\\');
                            s.push(other);
                        }
                        None => {}
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
        self.push(Tok::Str(s), line, col);
    }

    fn color(&mut self, line: u32, col: u32) {
        self.bump();
        let mut s = String::from("#");
        while let Some(c) = self.peek(0).filter(char::is_ascii_alphanumeric) {
            s.push(c);
            self.bump();
        }
        let hex = &s[1..];
        if !(matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())) {
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    Span::new(line, col, self.line, self.col),
                    format!("invalid color `{s}`"),
                )
                .with_help("colors are `#rgb`, `#rrggbb` or `#rrggbbaa`; comments start with `//`"),
            );
        }
        self.push(Tok::Color(s), line, col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        let (t, d) = lex(src);
        assert!(d.is_empty(), "{d:#?}");
        t.into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn lexes_statement_tokens() {
        assert_eq!(
            toks("box a-b \"x\" at db.top@0.5+(-1,2) fill=#fff // c\n"),
            vec![
                Tok::Ident("box".into()),
                Tok::Ident("a-b".into()),
                Tok::Str("x".into()),
                Tok::Ident("at".into()),
                Tok::Ident("db".into()),
                Tok::Dot,
                Tok::Ident("top".into()),
                Tok::At,
                Tok::Num(0.5),
                Tok::Plus,
                Tok::LParen,
                Tok::Num(-1.0),
                Tok::Comma,
                Tok::Num(2.0),
                Tok::RParen,
                Tok::Ident("fill".into()),
                Tok::Eq,
                Tok::Color("#fff".into()),
                Tok::Newline,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn arrow_is_not_part_of_an_identifier() {
        assert_eq!(
            toks("a->b"),
            vec![
                Tok::Ident("a".into()),
                Tok::Arrow,
                Tok::Ident("b".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn string_escapes_keep_markup_escapes() {
        assert_eq!(toks(r#""a\nb\"c\*""#)[0], Tok::Str("a\nb\"c\\*".into()));
    }

    #[test]
    fn spans_are_one_based() {
        let (t, _) = lex("  box");
        assert_eq!(
            (t[0].span.line, t[0].span.col, t[0].span.end_col),
            (1, 3, 6)
        );
    }

    #[test]
    fn hash_comment_is_an_invalid_color_with_help() {
        let (_, d) = lex("# note\n");
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("invalid color `#`"));
        assert!(d[0].help[0].contains("`//`"));
    }

    #[test]
    fn unterminated_string() {
        let (_, d) = lex("\"abc\nbox");
        assert_eq!(d[0].message, "unterminated string");
    }
}
