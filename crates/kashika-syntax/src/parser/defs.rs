use kashika_diag::{Diagnostic, Span, did_you_mean};

use super::{Clauses, PResult, Parser};
use crate::ast::*;
use crate::lexer::Tok;

const SHAPE_KEYS: &[&str] = &["path", "outline", "text-area"];

type PathBody = (
    Option<(String, Span)>,
    Option<(f64, Span)>,
    Option<([f64; 4], Span)>,
);

impl Parser {
    pub(super) fn import(&mut self, start: Span) -> PResult<Import> {
        let path_span = self.span();
        let path = self.string()?;
        let ns = if self.is_kw("as") {
            self.bump();
            Some(self.ident("a namespace name")?)
        } else {
            None
        };
        Ok(Import {
            path,
            path_span,
            ns,
            span: start.to(self.prev_span()),
        })
    }

    pub(super) fn image(&mut self, start: Span) -> PResult<Image> {
        let id = self.decl_id("an id")?;
        let file_span = self.span();
        let Tok::Str(file) = self.tok().clone() else {
            return self.error("a file path string");
        };
        self.bump();
        let tail = self.tail(Clauses::AT_SIZE)?;
        Ok(Image {
            id,
            file,
            file_span,
            at: self.required(tail.at, "at")?,
            size: self.required(tail.size, "size")?,
            attrs: tail.attrs,
            span: start.to(self.prev_span()),
        })
    }

    pub(super) fn shape_def(&mut self, start: Span) -> PResult<ShapeDef> {
        let name = self.ident("a shape name")?;
        if self.is_kw("from") {
            self.bump();
            let file_span = self.span();
            let file = self.string()?;
            let attrs = self.attrs(&[])?;
            return Ok(ShapeDef {
                name,
                body: ShapeBody::From {
                    file,
                    file_span,
                    attrs,
                },
                span: start.to(self.prev_span()),
            });
        }
        self.expect(Tok::LBrace)?;
        let Ok((path, outline, text_area)) = self.shape_body() else {
            self.skip_block();
            return Err(());
        };
        let span = start.to(self.prev_span());
        let Some(path) = path else {
            self.diags.push(
                Diagnostic::error(
                    "parse",
                    span,
                    format!("shape `{}` needs a `path`", name.name),
                )
                .with_help("e.g. `path \"M 0 0 L 1 0 L 1 1 L 0 1 Z\"`"),
            );
            return Err(());
        };
        Ok(ShapeDef {
            name,
            body: ShapeBody::Paths {
                path,
                outline,
                text_area,
            },
            span,
        })
    }

    /// Parses the items of a `shape { }` body up to and including the closing `}`.
    fn shape_body(&mut self) -> PResult<PathBody> {
        let (mut path, mut outline, mut text_area) = (None, None, None);
        loop {
            self.skip_newlines();
            match self.tok() {
                Tok::RBrace => {
                    self.bump();
                    return Ok((path, outline, text_area));
                }
                Tok::Eof => return self.error("`}`"),
                _ => {}
            }
            let key = self.ident("`path`, `outline` or `text-area`")?;
            let vstart = self.span();
            match key.name.as_str() {
                "path" => {
                    let s = self.string()?;
                    let span = vstart.to(self.prev_span());
                    set(&mut path, (s, span), &key, &mut self.diags);
                }
                "outline" => {
                    let n = self.number()?;
                    let span = vstart.to(self.prev_span());
                    set(&mut outline, (n, span), &key, &mut self.diags);
                }
                "text-area" => {
                    let q = self.quad()?;
                    let span = vstart.to(self.prev_span());
                    set(&mut text_area, (q, span), &key, &mut self.diags);
                }
                other => {
                    let mut d = Diagnostic::error(
                        "parse",
                        key.span,
                        format!(
                            "expected `path`, `outline` or `text-area` in a shape body, found `{other}`"
                        ),
                    );
                    if let Some(s) = did_you_mean(other, SHAPE_KEYS.iter().copied()) {
                        d = d.with_help(format!("did you mean `{s}`?"));
                    }
                    self.diags.push(d);
                    return Err(());
                }
            }
            if !matches!(self.tok(), Tok::Newline | Tok::RBrace) {
                return self.error("end of line");
            }
        }
    }

    fn quad(&mut self) -> PResult<[f64; 4]> {
        self.expect(Tok::LParen)?;
        let mut v = [0.0; 4];
        for (i, slot) in v.iter_mut().enumerate() {
            if i > 0 {
                self.expect(Tok::Comma)?;
            }
            *slot = self.number()?;
        }
        self.expect(Tok::RParen)?;
        Ok(v)
    }

    /// Skips to just past the `}` that closes the block whose `{` was already consumed.
    fn skip_block(&mut self) {
        let mut depth = 1usize;
        loop {
            match self.tok() {
                Tok::Eof => return,
                Tok::LBrace => depth += 1,
                Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.bump();
                        return;
                    }
                }
                _ => {}
            }
            self.bump();
        }
    }
}

fn set<T>(slot: &mut Option<T>, v: T, key: &Ident, diags: &mut Vec<Diagnostic>) {
    if slot.is_some() {
        diags.push(Diagnostic::error(
            "parse",
            key.span,
            format!("duplicate `{}` in shape body", key.name),
        ));
    } else {
        *slot = Some(v);
    }
}
