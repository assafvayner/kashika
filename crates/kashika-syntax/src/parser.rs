use std::cell::OnceCell;

use kashika_diag::{Diagnostic, Span, did_you_mean};

use crate::ast::*;
use crate::lexer::{Tok, Token, lex_raw};

mod defs;

const STATEMENTS: &[&str] = &[
    "diagram", "style", "box", "group", "junction", "text", "line", "import", "shape", "image",
];
const LINE_FLAGS: &[&str] = &["ortho"];
/// Segment keywords; an id with one of these names could not be a bare segment target.
pub const RESERVED_IDS: &[&str] = &["h", "v", "x", "y", "hv", "vh", "curve"];
const MAX_GROUP_DEPTH: usize = 64;

pub fn parse(src: &str) -> (File, Vec<Diagnostic>) {
    parse_stripped(src.strip_prefix('\u{feff}').unwrap_or(src))
}

/// Parses `src`, which has no byte order mark.
pub(crate) fn parse_stripped(src: &str) -> (File, Vec<Diagnostic>) {
    let (toks, _, mut diags) = lex_raw(src);
    let first_stmt = first_stmt(&toks);
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        diags: Vec::new(),
        src,
        lines: OnceCell::new(),
        skip_recover: false,
        declared: Vec::new(),
        failed_ids: Vec::new(),
        failed_diagram: false,
    };
    let stmts = p.stmts(false);
    diags.append(&mut p.diags);
    let file = File {
        stmts,
        first_stmt,
        failed_ids: p.failed_ids,
        failed_diagram: p.failed_diagram,
    };
    (file, diags)
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
    /// The size and the span of its clause.
    size: Option<(Size, Span)>,
    attrs: Vec<Attr>,
}

struct Parser<'a> {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
    diags: Vec<Diagnostic>,
    src: &'a str,
    lines: OnceCell<Vec<Vec<char>>>,
    skip_recover: bool,
    /// Ids read by the statements being parsed, kept for an enclosing group until it parses.
    declared: Vec<Ident>,
    failed_ids: Vec<Ident>,
    failed_diagram: bool,
}

impl Parser<'_> {
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

    fn bump(&mut self) -> Span {
        let span = self.toks[self.pos].span;
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        span
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
        let lines = self
            .lines
            .get_or_init(|| self.src.split('\n').map(|l| l.chars().collect()).collect());
        let mut out = String::new();
        for line in from.line..=to.end_line {
            let Some(chars) = lines.get(line as usize - 1) else {
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
                    if self.diags.last().is_none_or(|d| d.span != self.span()) {
                        self.diags
                            .push(Diagnostic::error("parse", self.span(), "unexpected `}`"));
                    }
                    self.bump();
                    continue;
                }
                _ => {}
            }
            match self.stmt() {
                Ok(Some(s)) => out.push(s),
                Ok(None) => {}
                Err(()) => {
                    if !std::mem::take(&mut self.skip_recover) {
                        self.recover();
                    }
                }
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

    /// Parses a statement; when it fails, records whether it was the `diagram` statement and
    /// the ids it declared, including those of a dropped group's children.
    fn stmt(&mut self) -> PResult<Option<Stmt>> {
        let mark = self.declared.len();
        let diagram = self.is_kw("diagram");
        let result = self.stmt_body();
        if result.is_err() {
            self.failed_diagram |= diagram;
            self.failed_ids.extend(self.declared.drain(mark..));
        } else if self.depth == 0 {
            self.declared.truncate(mark);
        }
        result
    }

    fn stmt_body(&mut self) -> PResult<Option<Stmt>> {
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
                let mut node = self.node(start, false)?;
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
                let node = self.node(start, true)?;
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
                let span = self.bump();
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
        self.declared.push(id.clone());
        Ok(id)
    }

    fn expect(&mut self, tok: Tok) -> PResult<Span> {
        if *self.tok() == tok {
            Ok(self.bump())
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

    /// `size (w,h)`, where each axis is a number or `auto`.
    fn size(&mut self) -> PResult<Size> {
        self.expect(Tok::LParen)?;
        let w = self.dim()?;
        self.expect(Tok::Comma)?;
        let h = self.dim()?;
        self.expect(Tok::RParen)?;
        Ok(Size { w, h })
    }

    fn dim(&mut self) -> PResult<Option<f64>> {
        if self.is_kw("auto") {
            self.bump();
            return Ok(None);
        }
        self.number().map(Some)
    }

    /// A required size with no `auto` axis, for statements that can't fit their size.
    fn fixed_size(&mut self, size: Option<(Size, Span)>, what: &str) -> PResult<(f64, f64)> {
        match self.required(size, "size")? {
            (
                Size {
                    w: Some(w),
                    h: Some(h),
                },
                _,
            ) => Ok((w, h)),
            (_, span) => {
                self.diags.push(
                    Diagnostic::error("parse", span, format!("{what} size can't be `auto`"))
                        .with_help(
                            "only boxes and the diagram take `auto`; give it a size: `size (w,h)`",
                        ),
                );
                Err(())
            }
        }
    }

    fn diagram(&mut self, start: Span) -> PResult<Diagram> {
        let title = self.string()?;
        let tail = self.tail(Clauses::SIZE)?;
        Ok(Diagram {
            title,
            size: tail.size.map(|(s, _)| s),
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
            match self.tok() {
                Tok::RBrace => {
                    self.bump();
                    break;
                }
                Tok::Eof => return self.error("`}`"),
                _ => {}
            }
            match self.attr() {
                Ok(a) => attrs.push(a),
                Err(()) => self.recover(),
            }
        }
        Ok(StyleDef {
            name: name.name,
            attrs,
            span: start.to(self.prev_span()),
        })
    }

    fn node(&mut self, start: Span, group: bool) -> PResult<Node> {
        let id = self.decl_id("an id")?;
        let label = if let Tok::Str(s) = self.tok().clone() {
            self.bump();
            Some(s)
        } else {
            None
        };
        let tail = self.tail(Clauses::AT_SIZE)?;
        let at = self.required(tail.at, "at")?;
        let size = if group {
            let (w, h) = self.fixed_size(tail.size, "a group's")?;
            Some(Size {
                w: Some(w),
                h: Some(h),
            })
        } else {
            tail.size.map(|(s, _)| s)
        };
        Ok(Node {
            id,
            label,
            at,
            size,
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
            let colon = self.bump();
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
            let arrow = self.bump();
            if matches!(self.tok(), Tok::Newline | Tok::Eof) {
                return self.dangling_arrow(arrow);
            }
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

    /// Reports a `->` that ends its line. A next line that is only a segment target is
    /// swallowed with it, so one mistake is one error.
    fn dangling_arrow<T>(&mut self, arrow: Span) -> PResult<T> {
        self.diags.push(
            Diagnostic::error(
                "parse",
                arrow,
                "`->` must be followed by a segment on the same line",
            )
            .with_help(
                "to continue a line, start the next physical line with `->`, \
                 e.g. `line a x.right` then `  -> y.left`",
            ),
        );
        let mut n = 0;
        while *self.tok_at(n) == Tok::Newline {
            n += 1;
        }
        let target = match self.tok_at(n) {
            Tok::LParen => true,
            Tok::Ident(s) => {
                !STATEMENTS.contains(&s.as_str())
                    && matches!(
                        self.tok_at(n + 1),
                        Tok::Dot | Tok::Plus | Tok::Minus | Tok::Newline | Tok::Eof
                    )
            }
            _ => false,
        };
        if target {
            for _ in 0..n {
                self.bump();
            }
            self.recover();
            self.skip_recover = true;
        }
        Err(())
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
                let kw = self.bump();
                let at = self.point()?;
                self.once(&mut tail.at, at, "at", kw)?;
            } else if clauses.size && clause && self.is_kw("size") {
                let kw = self.bump();
                let size = self.size()?;
                let span = kw.to(self.prev_span());
                self.once(&mut tail.size, (size, span), "size", kw)?;
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
        if *self.tok() == Tok::LParen && self.paren_is_group() {
            return self.group();
        }
        self.bare_point()
    }

    /// A literal, mixed or reference point, without grouping parentheses.
    fn bare_point(&mut self) -> PResult<Point> {
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
            let neg = *self.tok() == Tok::Minus;
            self.bump();
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

    /// Whether the `(` here opens a grouped point rather than `(x,y)`: it is followed by `(`,
    /// or by a reference with no `,` before its closing `)`.
    fn paren_is_group(&self) -> bool {
        match self.tok_at(1) {
            Tok::LParen => return true,
            Tok::Ident(_) => {}
            _ => return false,
        }
        let mut depth = 0usize;
        let mut n = 2;
        loop {
            match self.tok_at(n) {
                Tok::LParen => depth += 1,
                Tok::RParen if depth == 0 => return true,
                Tok::RParen => depth -= 1,
                Tok::Comma if depth == 0 => return false,
                Tok::Newline | Tok::Eof => return true,
                _ => {}
            }
            n += 1;
        }
    }

    fn group(&mut self) -> PResult<Point> {
        if *self.tok_at(1) == Tok::LParen {
            return self.nested_group();
        }
        let open = self.bump();
        let inner = self.bare_point()?;
        if *self.tok() != Tok::RParen {
            let found = self.tok().describe();
            let written = self.source(open, inner.span());
            let help = if *self.tok() == Tok::Dot
                && matches!(self.tok_at(1), Tok::Ident(a) if a == "x" || a == "y")
            {
                format!(
                    "`.x`/`.y` take one axis of a mixed point, which needs both: `{written}.x, other.y)`"
                )
            } else {
                format!("close it: `{written})`")
            };
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    self.span(),
                    format!("expected `)` to close the `(` around the point, found {found}"),
                )
                .with_help(help),
            );
            return Err(());
        }
        self.bump();
        let span = open.to(self.prev_span());
        if matches!(self.tok(), Tok::Plus | Tok::Minus) && *self.tok_at(1) == Tok::LParen {
            let sign = self.bump();
            self.pair()?;
            let offset = sign.to(self.prev_span());
            let help = match inner.ungrouped() {
                p @ Point::Ref { offset: None, .. } => format!(
                    "put the offset inside the parentheses: `({}{})`",
                    self.source(p.span(), p.span()),
                    self.source(offset, offset)
                ),
                _ => "put the offset inside the parentheses, e.g. `(db.center+(0,40))`".into(),
            };
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    offset,
                    "an offset cannot follow a parenthesized point",
                )
                .with_help(help),
            );
            return Err(());
        }
        Ok(Point::Group {
            inner: Box::new(inner),
            span,
        })
    }

    /// Reports a point in more than one set of parentheses in one pass, without recursing.
    /// The help peels each leading `(` whose `)` sits right inside the enclosing one's.
    fn nested_group(&mut self) -> PResult<Point> {
        let start = self.pos;
        let rest = &self.toks[start..];
        let leading = rest.iter().take_while(|t| t.tok == Tok::LParen).count();
        let mut close = vec![None; leading];
        let mut depth = 0usize;
        for (i, t) in rest.iter().enumerate() {
            match t.tok {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    depth -= 1;
                    if let Some(c @ None) = close.get_mut(depth) {
                        *c = Some(start + i);
                    }
                    if depth == 0 {
                        break;
                    }
                }
                Tok::Newline | Tok::Eof => break,
                _ => {}
            }
        }
        let mut k = 0;
        while k + 1 < leading
            && matches!((close[k], close[k + 1]), (Some(a), Some(b)) if b + 1 == a)
        {
            k += 1;
        }
        let help = match close[k] {
            Some(end) if k > 0 && self.toks[start + k + 1].tok != Tok::LParen => format!(
                "remove the extra parentheses: `{}`",
                self.source(self.toks[start + k].span, self.toks[end].span)
            ),
            _ => "keep one set of parentheses around the point, e.g. `(db.center+(0,40))`".into(),
        };
        let span = match close[0] {
            Some(end) => self.span().to(self.toks[end].span),
            None => self.span(),
        };
        self.diags.push(
            Diagnostic::error(
                "parse",
                span,
                "a point takes at most one set of parentheses",
            )
            .with_help(help),
        );
        Err(())
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
        let side_span = self.bump();
        let colon = (*self.tok() == Tok::Colon).then(|| self.bump());
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
