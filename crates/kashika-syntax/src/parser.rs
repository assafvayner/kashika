use kashika_diag::{Diagnostic, Span, did_you_mean};

use crate::ast::*;
use crate::lexer::{Tok, Token, lex};

mod defs;

const STATEMENTS: &[&str] = &[
    "diagram", "style", "box", "group", "junction", "text", "line", "import", "shape", "image",
];
const LINE_FLAGS: &[&str] = &["ortho"];
/// Segment keywords; an id with one of these names could not be a bare segment target.
pub const RESERVED_IDS: &[&str] = &["h", "v", "x", "y", "hv", "vh", "curve"];
pub const MAX_GROUP_DEPTH: usize = 64;

pub fn parse(src: &str) -> (File, Vec<Diagnostic>) {
    let (toks, mut diags) = lex(src);
    let first_stmt = first_stmt(&toks);
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        diags: Vec::new(),
        lines: src.split('\n').map(|l| l.chars().collect()).collect(),
    };
    let stmts = p.stmts(false);
    diags.append(&mut p.diags);
    (File { stmts, first_stmt }, diags)
}

/// Where the first statement starts, skipping a misplaced `kashika=` line, which is
/// reported on its own.
fn first_stmt(toks: &[Token]) -> Option<Span> {
    let mut i = 0;
    loop {
        match &toks.get(i)?.tok {
            Tok::Newline => i += 1,
            Tok::Eof => return None,
            Tok::Ident(s) if s == "kashika" && toks.get(i + 1)?.tok == Tok::Eq => {
                while !matches!(toks.get(i)?.tok, Tok::Newline | Tok::Eof) {
                    i += 1;
                }
            }
            _ => return Some(toks[i].span),
        }
    }
}

fn starts_point(t: &Tok) -> bool {
    matches!(t, Tok::Ident(_) | Tok::LParen)
}

type PResult<T> = Result<T, ()>;

#[derive(Clone, Copy)]
struct Clauses {
    at: bool,
    size: bool,
}

impl Clauses {
    const AT: Self = Self {
        at: true,
        size: false,
    };
    const SIZE: Self = Self {
        at: false,
        size: true,
    };
    const AT_SIZE: Self = Self {
        at: true,
        size: true,
    };
}

#[derive(Default)]
struct Tail {
    at: Option<Point>,
    size: Option<(f64, f64)>,
    attrs: Vec<Attr>,
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
    diags: Vec<Diagnostic>,
    lines: Vec<Vec<char>>,
}

impl Parser {
    fn tok(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn tok_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn error<T>(&mut self, expected: &str) -> PResult<T> {
        let found = self.tok().describe();
        self.diags.push(Diagnostic::error(
            "parse",
            self.span(),
            format!("expected {expected}, found {found}"),
        ));
        Err(())
    }

    /// The source text from the start of `from` to the end of `to`.
    fn source(&self, from: Span, to: Span) -> String {
        let mut out = String::new();
        for line in from.line..=to.end_line {
            let Some(chars) = self.lines.get(line as usize - 1) else {
                break;
            };
            let a = if line == from.line {
                from.col as usize - 1
            } else {
                0
            };
            let b = if line == to.end_line {
                to.end_col as usize - 1
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

    /// The span of the last token before the end of the current physical line.
    fn line_end(&self) -> Span {
        let mut i = self.pos;
        while i + 1 < self.toks.len() && !matches!(self.toks[i + 1].tok, Tok::Newline | Tok::Eof) {
            i += 1;
        }
        self.toks[i].span
    }

    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.tok(), Tok::Ident(s) if s == kw)
    }

    fn skip_newlines(&mut self) {
        while *self.tok() == Tok::Newline {
            self.bump();
        }
    }

    fn stmts(&mut self, in_block: bool) -> Vec<Stmt> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            match self.tok() {
                Tok::Eof => {
                    if in_block {
                        let _: PResult<()> = self.error("`}`");
                    }
                    break;
                }
                Tok::RBrace if in_block => break,
                Tok::RBrace => {
                    self.diags
                        .push(Diagnostic::error("parse", self.span(), "unexpected `}`"));
                    self.bump();
                    continue;
                }
                _ => {}
            }
            match self.stmt() {
                Ok(Some(s)) => out.push(s),
                Ok(None) => {}
                Err(()) => self.recover(),
            }
        }
        out
    }

    /// Skips the rest of a broken statement: up to the next newline at brace depth 0,
    /// or up to (not including) a `}` that closes the enclosing block.
    fn recover(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.tok() {
                Tok::Eof => return,
                Tok::Newline if depth == 0 => {
                    self.bump();
                    return;
                }
                Tok::RBrace if depth == 0 => return,
                Tok::LBrace => depth += 1,
                Tok::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    fn stmt(&mut self) -> PResult<Option<Stmt>> {
        let start = self.span();
        let Tok::Ident(kw) = self.tok().clone() else {
            return self.error("a statement");
        };
        if kw == "kashika" && *self.tok_at(1) == Tok::Eq {
            let end = self.line_end();
            let version = match self.tok_at(2) {
                Tok::Str(_) => self.source(self.toks[self.pos + 2].span, end),
                _ => "\"X.Y.Z\"".to_string(),
            };
            self.diags.push(
                Diagnostic::error("parse", start.to(end), "`kashika=` is not a statement")
                    .with_help(format!(
                        "the version is an attribute of `diagram`: \
                         write `diagram \"…\" size (w,h) kashika={version}`"
                    )),
            );
            return Err(());
        }
        let stmt = match kw.as_str() {
            "diagram" => {
                self.bump();
                Stmt::Diagram(self.diagram(start)?)
            }
            "style" => {
                self.bump();
                Stmt::Style(self.style_def(start)?)
            }
            "box" => {
                self.bump();
                let mut node = self.node(start)?;
                if *self.tok() == Tok::LBrace {
                    node.block = Some(self.text_block()?);
                }
                Stmt::Box(node)
            }
            "group" if self.depth >= MAX_GROUP_DEPTH => {
                let closed = self.skip_unbuilt();
                self.diags.push(
                    Diagnostic::error(
                        "parse",
                        start.to(self.prev_span()),
                        format!("groups are nested more than {MAX_GROUP_DEPTH} deep"),
                    )
                    .with_help("flatten the hierarchy; ids are global, so children need no parent"),
                );
                if !closed {
                    return self.error("`}`");
                }
                self.end_of_stmt()?;
                return Ok(None);
            }
            "group" => {
                self.bump();
                let node = self.node(start)?;
                self.depth += 1;
                let children = self.block();
                self.depth -= 1;
                let children = children?;
                Stmt::Group(Group { node, children })
            }
            "junction" => {
                self.bump();
                Stmt::Junction(self.junction(start)?)
            }
            "text" => {
                self.bump();
                Stmt::Text(self.text(start)?)
            }
            "line" => {
                self.bump();
                Stmt::Line(self.line(start)?)
            }
            "import" => {
                self.bump();
                Stmt::Import(self.import(start)?)
            }
            "shape" => {
                self.bump();
                Stmt::Shape(self.shape_def(start)?)
            }
            "image" => {
                self.bump();
                Stmt::Image(self.image(start)?)
            }
            _ => {
                let mut d = Diagnostic::error(
                    "parse",
                    self.span(),
                    format!(
                        "expected a statement ({}), found `{kw}`",
                        STATEMENTS.join(", ")
                    ),
                );
                if let Some(s) = did_you_mean(&kw, STATEMENTS.iter().copied()) {
                    d = d.with_help(format!("did you mean `{s}`?"));
                }
                self.diags.push(d);
                return Err(());
            }
        };
        self.end_of_stmt()?;
        Ok(Some(stmt))
    }

    /// Skips a statement the parser won't build: up to the end of the line, or past the
    /// `}` that matches any `{` it opens, across lines.
    /// Returns `false` if the input ends inside an open `{`.
    fn skip_unbuilt(&mut self) -> bool {
        let mut depth = 0usize;
        loop {
            match self.tok() {
                Tok::Eof => return depth == 0,
                Tok::Newline | Tok::RBrace if depth == 0 => return true,
                Tok::LBrace => depth += 1,
                Tok::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    fn end_of_stmt(&mut self) -> PResult<()> {
        match self.tok() {
            Tok::Newline => {
                self.bump();
                Ok(())
            }
            Tok::Eof | Tok::RBrace => Ok(()),
            _ => self.error("end of statement"),
        }
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        if *self.tok() != Tok::LBrace {
            return self.error("`{`");
        }
        self.bump();
        let children = self.stmts(true);
        if *self.tok() == Tok::RBrace {
            self.bump();
        }
        Ok(children)
    }

    fn ident(&mut self, what: &str) -> PResult<Ident> {
        match self.tok().clone() {
            Tok::Ident(name) => {
                let span = self.bump().span;
                Ok(Ident { name, span })
            }
            _ => self.error(what),
        }
    }

    /// An id being declared; segment keywords are reserved, and a statement declaring one
    /// is dropped.
    fn decl_id(&mut self, what: &str) -> PResult<Ident> {
        let id = self.ident(what)?;
        if RESERVED_IDS.contains(&id.name.as_str()) {
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    id.span,
                    format!("`{}` is a reserved word and cannot be an id", id.name),
                )
                .with_help(format!(
                    "segment keywords ({}) are reserved; pick another name, e.g. `{}_1`",
                    RESERVED_IDS.join(" "),
                    id.name
                )),
            );
            return Err(());
        }
        Ok(id)
    }

    fn expect(&mut self, tok: Tok) -> PResult<Span> {
        if *self.tok() == tok {
            Ok(self.bump().span)
        } else {
            self.error(&tok.describe())
        }
    }

    fn number(&mut self) -> PResult<f64> {
        if let Tok::Num(n) = self.tok() {
            let n = *n;
            self.bump();
            Ok(n)
        } else {
            self.error("a number")
        }
    }

    fn string(&mut self) -> PResult<String> {
        match self.tok().clone() {
            Tok::Str(s) => {
                self.bump();
                Ok(s)
            }
            _ => self.error("a string"),
        }
    }

    fn pair(&mut self) -> PResult<(f64, f64)> {
        self.expect(Tok::LParen)?;
        let a = self.number()?;
        self.expect(Tok::Comma)?;
        let b = self.number()?;
        self.expect(Tok::RParen)?;
        Ok((a, b))
    }

    fn diagram(&mut self, start: Span) -> PResult<Diagram> {
        let title = self.string()?;
        let tail = self.tail(Clauses::SIZE)?;
        Ok(Diagram {
            title,
            size: self.required(tail.size, "size")?,
            attrs: tail.attrs,
            span: start.to(self.prev_span()),
        })
    }

    fn style_def(&mut self, start: Span) -> PResult<StyleDef> {
        let name = self.ident("a style name")?;
        self.expect(Tok::LBrace)?;
        let mut attrs = Vec::new();
        loop {
            self.skip_newlines();
            if *self.tok() == Tok::RBrace {
                self.bump();
                break;
            }
            attrs.push(self.attr()?);
        }
        Ok(StyleDef {
            name: name.name,
            attrs,
            span: start.to(self.prev_span()),
        })
    }

    fn node(&mut self, start: Span) -> PResult<Node> {
        let id = self.decl_id("an id")?;
        let label = if let Tok::Str(s) = self.tok().clone() {
            self.bump();
            Some(s)
        } else {
            None
        };
        let tail = self.tail(Clauses::AT_SIZE)?;
        Ok(Node {
            id,
            label,
            at: self.required(tail.at, "at")?,
            size: self.required(tail.size, "size")?,
            attrs: tail.attrs,
            block: None,
            span: start.to(self.prev_span()),
        })
    }

    /// Parses a box's `{ }` text block: `text "<content>" [at <point>] [attrs]` per line.
    fn text_block(&mut self) -> PResult<Vec<BlockText>> {
        self.bump();
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            match self.tok() {
                Tok::RBrace => {
                    self.bump();
                    return Ok(items);
                }
                Tok::Eof => return self.error("`}`"),
                _ => {}
            }
            match self.block_text() {
                Ok(t) => items.push(t),
                Err(()) => self.recover(),
            }
        }
    }

    fn block_text(&mut self) -> PResult<BlockText> {
        let start = self.span();
        if !self.is_kw("text") {
            return self.error("`text` or `}` in a text block");
        }
        self.bump();
        if let Tok::Ident(name) = self.tok().clone() {
            self.diags.push(
                Diagnostic::error("parse", self.span(), "text block items take no id")
                    .with_help(format!("remove `{name}`")),
            );
            return Err(());
        }
        let content = self.string()?;
        let tail = self.tail(Clauses::AT)?;
        let span = start.to(self.prev_span());
        match self.tok() {
            Tok::Newline | Tok::RBrace => Ok(BlockText {
                content,
                at: tail.at,
                attrs: tail.attrs,
                span,
            }),
            _ => self.error("end of statement"),
        }
    }

    fn junction(&mut self, start: Span) -> PResult<Junction> {
        let id = self.decl_id("an id")?;
        let tail = self.tail(Clauses::AT)?;
        Ok(Junction {
            id,
            at: self.required(tail.at, "at")?,
            attrs: tail.attrs,
            span: start.to(self.prev_span()),
        })
    }

    fn text(&mut self, start: Span) -> PResult<Text> {
        let id = if matches!(self.tok(), Tok::Ident(_)) {
            Some(self.decl_id("an id")?)
        } else {
            None
        };
        let content = self.string()?;
        let tail = self.tail(Clauses::AT)?;
        Ok(Text {
            id,
            content,
            at: self.required(tail.at, "at")?,
            attrs: tail.attrs,
            span: start.to(self.prev_span()),
        })
    }

    /// A line id is an identifier followed by something that starts a point; a point that
    /// starts with an identifier never continues with another identifier or `(`.
    fn line(&mut self, start: Span) -> PResult<Line> {
        let mut colon_diag = None;
        let mut id = if matches!(self.tok(), Tok::Ident(_)) && *self.tok_at(1) == Tok::Colon {
            let id = self.decl_id("a line id")?;
            let colon = self.bump().span;
            let rest = if matches!(self.tok(), Tok::Newline | Tok::Eof) {
                "a.right -> b.left".to_string()
            } else {
                self.source(self.span(), self.line_end())
            };
            colon_diag = Some(self.diags.len());
            self.diags.push(
                Diagnostic::error("parse", colon, "a line id takes no `:`")
                    .with_help(format!("remove the `:`: write `line {} {rest}`", id.name)),
            );
            Some(id)
        } else if matches!(self.tok(), Tok::Ident(_)) && starts_point(self.tok_at(1)) {
            Some(self.decl_id("a line id")?)
        } else {
            None
        };
        let from_kw = self.is_kw("from") && starts_point(self.tok_at(1));
        if from_kw {
            self.bump();
        }
        let start_pt = self.point()?;
        let to_kw = self.is_kw("to") && starts_point(self.tok_at(1));
        if from_kw || to_kw {
            if !from_kw && id.as_ref().is_some_and(|i| i.name == "from") {
                id = None;
            }
            let a = self.source(start_pt.span(), start_pt.span());
            let b = if to_kw {
                self.bump();
                let end = self.point()?;
                self.source(end.span(), end.span())
            } else {
                "b.left".to_string()
            };
            if let Some(i) = colon_diag {
                self.diags.remove(i);
            }
            let id = id.map(|i| format!("{} ", i.name)).unwrap_or_default();
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    start.to(self.prev_span()),
                    "a line is written with `->`, not `from … to`",
                )
                .with_help(format!("write `line {id}{a} -> {b}`")),
            );
            return Err(());
        }
        let mut segs = Vec::new();
        loop {
            self.continuation(true, LINE_FLAGS);
            if *self.tok() != Tok::Arrow {
                break;
            }
            self.bump();
            segs.push(self.seg()?);
        }
        if segs.is_empty() {
            return self.error("`->` followed by a segment");
        }
        let attrs = self.attrs(LINE_FLAGS)?;
        Ok(Line {
            id,
            start: start_pt,
            segs,
            attrs,
            span: start.to(self.prev_span()),
        })
    }

    /// Joins the next physical line onto this statement when it starts with `->`
    /// (if `allow_arrow`), with an attribute (`key=` or `key@`), or with one of `flags`.
    fn continuation(&mut self, allow_arrow: bool, flags: &[&str]) {
        if *self.tok() != Tok::Newline {
            return;
        }
        let mut n = 0;
        while *self.tok_at(n) == Tok::Newline {
            n += 1;
        }
        let continues = (allow_arrow && *self.tok_at(n) == Tok::Arrow)
            || (matches!(self.tok_at(n), Tok::Ident(_))
                && matches!(self.tok_at(n + 1), Tok::Eq | Tok::At))
            || matches!(self.tok_at(n), Tok::Ident(s) if flags.contains(&s.as_str()));
        if continues {
            for _ in 0..n {
                self.bump();
            }
        }
    }

    /// Parses `at <point>` and `size (w,h)`, where `clauses` allows them, and attributes,
    /// in any order. A repeated `at` or `size` is an error.
    fn tail(&mut self, clauses: Clauses) -> PResult<Tail> {
        let mut tail = Tail::default();
        loop {
            self.continuation(false, &[]);
            let clause = !matches!(self.tok_at(1), Tok::Eq | Tok::At);
            if clauses.at && clause && self.is_kw("at") {
                let kw = self.bump().span;
                let at = self.point()?;
                self.once(&mut tail.at, at, "at", kw)?;
            } else if clauses.size && clause && self.is_kw("size") {
                let kw = self.bump().span;
                let size = self.pair()?;
                self.once(&mut tail.size, size, "size", kw)?;
            } else if matches!(self.tok(), Tok::Ident(_)) {
                tail.attrs.push(self.attr()?);
            } else {
                return Ok(tail);
            }
        }
    }

    fn once<T>(&mut self, slot: &mut Option<T>, v: T, kw: &str, span: Span) -> PResult<()> {
        if slot.is_some() {
            self.diags.push(
                Diagnostic::error("parse", span, format!("duplicate `{kw}`"))
                    .with_help(format!("a statement takes one `{kw}`; remove the other")),
            );
            return Err(());
        }
        *slot = Some(v);
        Ok(())
    }

    fn required<T>(&mut self, v: Option<T>, kw: &str) -> PResult<T> {
        match v {
            Some(v) => Ok(v),
            None => self.error(&format!("`{kw}`")),
        }
    }

    fn attrs(&mut self, flags: &[&str]) -> PResult<Vec<Attr>> {
        let mut out = Vec::new();
        loop {
            self.continuation(false, flags);
            if !matches!(self.tok(), Tok::Ident(_)) {
                return Ok(out);
            }
            out.push(self.attr()?);
        }
    }

    fn attr(&mut self) -> PResult<Attr> {
        let key = self.ident("an attribute")?;
        let at_t = if *self.tok() == Tok::At {
            self.bump();
            Some(self.number()?)
        } else {
            None
        };
        let value = if *self.tok() == Tok::Eq {
            self.bump();
            let value = self.value()?;
            if key.name == "pin" && *self.tok() == Tok::At {
                self.bump();
                let t = self.number()?;
                match value {
                    Value::Word(w) => Value::AnchorAt(w, t),
                    _ => return self.error("an anchor name before `@`"),
                }
            } else {
                value
            }
        } else if at_t.is_some() {
            return self.error("`=`");
        } else {
            Value::Flag
        };
        let span = key.span.to(self.prev_span());
        Ok(Attr {
            key,
            at_t,
            value,
            span,
        })
    }

    fn value(&mut self) -> PResult<Value> {
        match self.tok().clone() {
            Tok::Num(n) => {
                self.bump();
                Ok(Value::Num(n))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Value::Str(s))
            }
            Tok::Color(c) => {
                self.bump();
                Ok(Value::Color(c))
            }
            Tok::LParen => {
                let (a, b) = self.pair()?;
                Ok(Value::Pair(a, b))
            }
            Tok::Ident(first) => {
                self.bump();
                let mut word = first;
                loop {
                    let sep = match self.tok() {
                        Tok::Colon => ':',
                        Tok::Comma => ',',
                        Tok::Dot if matches!(self.tok_at(1), Tok::Ident(_)) => '.',
                        _ => break,
                    };
                    self.bump();
                    match self.tok().clone() {
                        Tok::Ident(s) => {
                            self.bump();
                            word.push(sep);
                            word.push_str(&s);
                        }
                        _ => return self.error("an identifier"),
                    }
                }
                Ok(Value::Word(word))
            }
            _ => self.error("an attribute value"),
        }
    }

    fn point(&mut self) -> PResult<Point> {
        let start = self.span();
        if *self.tok() == Tok::LParen {
            self.bump();
            let x = self.coord()?;
            self.expect(Tok::Comma)?;
            let y = self.coord()?;
            self.expect(Tok::RParen)?;
            return Ok(Point::Lit {
                x,
                y,
                span: start.to(self.prev_span()),
            });
        }
        let r = self.reference()?;
        let offset = if matches!(self.tok(), Tok::Plus | Tok::Minus) {
            let neg = self.bump().tok == Tok::Minus;
            let (dx, dy) = self.pair()?;
            Some(Offset { neg, dx, dy })
        } else {
            None
        };
        Ok(Point::Ref {
            r,
            offset,
            span: start.to(self.prev_span()),
        })
    }

    fn coord(&mut self) -> PResult<Coord> {
        if let Tok::Num(n) = self.tok() {
            let n = *n;
            self.bump();
            return Ok(Coord::Num(n));
        }
        if !matches!(self.tok(), Tok::Ident(_)) {
            return self.error("a number or a reference like `db.left.x`");
        }
        let r = self.reference()?;
        self.expect(Tok::Dot)?;
        let axis = match self.tok() {
            Tok::Ident(s) if s == "x" => Axis::X,
            Tok::Ident(s) if s == "y" => Axis::Y,
            _ => return self.error("`x` or `y`"),
        };
        self.bump();
        Ok(Coord::Axis { r, axis })
    }

    fn reference(&mut self) -> PResult<Ref> {
        let id = self.ident("a point: `(x,y)` or a reference like `db.left`")?;
        let next_is_anchor = *self.tok() == Tok::Dot
            && matches!(self.tok_at(1), Tok::Ident(s) if s != "x" && s != "y");
        let anchor = if next_is_anchor {
            self.bump();
            let name = self.ident("an anchor")?;
            let t = if *self.tok() == Tok::At {
                self.bump();
                Some(self.number()?)
            } else {
                None
            };
            Some(Anchor { name, t })
        } else {
            None
        };
        Ok(Ref { id, anchor })
    }

    fn seg(&mut self) -> PResult<Seg> {
        let start = self.span();
        let kw = match self.tok() {
            Tok::Ident(s) if *self.tok_at(1) != Tok::Dot => Some(s.clone()),
            _ => None,
        };
        let kind = match kw.as_deref() {
            Some("h") => {
                self.bump();
                SegKind::H(self.number()?)
            }
            Some("v") => {
                self.bump();
                SegKind::V(self.number()?)
            }
            Some("x") => {
                self.bump();
                SegKind::X(self.target()?)
            }
            Some("y") => {
                self.bump();
                SegKind::Y(self.target()?)
            }
            Some("hv") => {
                self.bump();
                SegKind::Hv(self.point()?)
            }
            Some("vh") => {
                self.bump();
                SegKind::Vh(self.point()?)
            }
            Some("curve") => {
                self.bump();
                self.curve()?
            }
            _ => SegKind::To(self.point()?),
        };
        Ok(Seg {
            kind,
            span: start.to(self.prev_span()),
        })
    }

    fn target(&mut self) -> PResult<Target> {
        if let Tok::Num(n) = self.tok() {
            let n = *n;
            self.bump();
            Ok(Target::Num(n))
        } else {
            Ok(Target::Point(self.point()?))
        }
    }

    fn curve(&mut self) -> PResult<SegKind> {
        if self.is_kw("via") {
            self.bump();
            let c1 = self.point()?;
            let c2 = self.point()?;
            let to = self.point()?;
            return Ok(SegKind::CurveVia { c1, c2, to });
        }
        let side = if self.is_kw("left") {
            Side::Left
        } else if self.is_kw("right") {
            Side::Right
        } else {
            return self.error("`left <N>`, `right <N>` or `via`");
        };
        let side_span = self.bump().span;
        let colon = (*self.tok() == Tok::Colon).then(|| self.bump().span);
        let amount_span = self.span();
        let amount = self.number();
        let to = amount.and_then(|_| self.point());
        if let Some(colon) = colon {
            let side = self.source(side_span, side_span);
            let rest = match &to {
                Ok(p) => self.source(amount_span, p.span()),
                Err(()) => "<N> <point>".to_string(),
            };
            self.diags.push(
                Diagnostic::error("parse", colon, "`curve left` and `curve right` take no `:`")
                    .with_help(format!("remove the `:`: write `curve {side} {rest}`")),
            );
        }
        Ok(SegKind::Curve {
            side,
            amount: amount?,
            to: to?,
        })
    }
}
