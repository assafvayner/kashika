use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use kashika_diag::{Diagnostic, Span};
use kashika_geom::{parse_unit_path, Rect};
use kashika_syntax::ast::{self, ShapeBody, Stmt};

use crate::assets::Store;
use crate::attrs::{self, Attrs};
use crate::loader::{FileError, Found, Loader};
use crate::shapes::{UnitShape, BUILTIN_SHAPES};
use crate::style::{StyleProps, STYLE_KEYS};

/// Deepest chain of nested imports followed before giving up.
const MAX_IMPORT_DEPTH: usize = 32;
/// Import chains longer than this print only their first two and last hops.
const HOPS_SHOWN: usize = 4;

#[derive(Debug, Clone)]
pub(crate) enum LibShape {
    Unit(UnitShape),
    Svg(SvgShape),
}

impl LibShape {
    /// The unit text area before padding; `None` when the text sits below the shape.
    pub fn text_area(&self) -> Option<Rect> {
        match self {
            LibShape::Unit(u) => Some(u.text_area),
            LibShape::Svg(s) if s.below => None,
            LibShape::Svg(_) => Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
        }
    }
}

/// A shape drawn from an SVG file.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SvgShape {
    /// Index into the asset store.
    pub asset: usize,
    /// `outline=hull`: the drawing's convex hull instead of the box.
    pub hull: bool,
    /// `text-area=below`.
    pub below: bool,
}

/// Shape and style definitions from the main file and its imports, by qualified name.
#[derive(Default)]
pub(crate) struct Library {
    pub shapes: BTreeMap<String, LibShape>,
    /// Imported styles in load order, with qualified names. The main file's own styles
    /// are read by the resolver.
    pub styles: Vec<(String, StyleProps)>,
    /// (canonical path, namespace prefix) of every library loaded so far.
    loaded: HashSet<(PathBuf, String)>,
}

pub(crate) struct Ctx<'l> {
    pub loader: &'l Loader,
    pub store: &'l mut Store,
    pub diags: Vec<Diagnostic>,
    /// Diagnostics from imported files, not yet prefixed.
    pub nested: Vec<Nested>,
}

/// A diagnostic from inside an import, with the chain of imports that leads to it.
pub(crate) struct Nested {
    /// The import's path string in the file being processed.
    at: Span,
    /// (import path, line, column) per file along the chain; the last is where `diag` is.
    hops: Vec<(String, u32, u32)>,
    diag: Diagnostic,
}

impl Nested {
    fn finish(self) -> Diagnostic {
        let hop = |(path, line, col): &(String, u32, u32)| format!("in `{path}` at {line}:{col}");
        let n = self.hops.len();
        let prefix: Vec<String> = if n > HOPS_SHOWN {
            vec![
                hop(&self.hops[0]),
                hop(&self.hops[1]),
                format!("… {} more imports …", n - 3),
                hop(&self.hops[n - 1]),
            ]
        } else {
            self.hops.iter().map(hop).collect()
        };
        Diagnostic {
            span: self.at,
            message: format!("{}: {}", prefix.join(": "), self.diag.message),
            elements: Vec::new(),
            suggestions: Vec::new(),
            ..self.diag
        }
    }
}

impl Library {
    pub fn load(file: &ast::File, ctx: &mut Ctx) -> Library {
        let mut lib = Library::default();
        let root = ctx.loader.root().to_path_buf();
        check_import_order(file, &mut ctx.diags);
        let mut stack: Vec<PathBuf> = ctx
            .loader
            .main()
            .map(Path::to_path_buf)
            .into_iter()
            .collect();
        lib.defs(&file.stmts, &root, "", &mut stack, ctx);
        finish(ctx);
        lib
    }

    /// Loads a library file checked on its own, as if it were imported without `as`.
    pub fn load_standalone(file: &ast::File, ctx: &mut Ctx) -> Library {
        let mut lib = Library::default();
        let root = ctx.loader.root().to_path_buf();
        let mut stack: Vec<PathBuf> = ctx
            .loader
            .main()
            .map(Path::to_path_buf)
            .into_iter()
            .collect();
        for stmt in &file.stmts {
            if let Stmt::Style(s) = stmt {
                lib.style(s, "", ctx);
            }
        }
        lib.defs(&file.stmts, &root, "", &mut stack, ctx);
        finish(ctx);
        lib
    }

    fn defs(
        &mut self,
        stmts: &[Stmt],
        dir: &Path,
        prefix: &str,
        stack: &mut Vec<PathBuf>,
        ctx: &mut Ctx,
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::Shape(s) => self.shape(s, dir, prefix, ctx),
                Stmt::Import(i) => self.import(i, dir, prefix, stack, ctx),
                _ => {}
            }
        }
    }

    fn import(
        &mut self,
        i: &ast::Import,
        dir: &Path,
        prefix: &str,
        stack: &mut Vec<PathBuf>,
        ctx: &mut Ctx,
    ) {
        let fail = |ctx: &mut Ctx, e: FileError| push_file_error(&mut ctx.diags, i.path_span, e);
        let path = match ctx.loader.find(dir, &i.path) {
            Ok(Found::File(p)) => p,
            Ok(Found::Bundled(_)) => {
                return fail(
                    ctx,
                    FileError::new(format!("`{}` is a bundled icon, not a library", i.path)),
                )
            }
            Err(e) => return fail(ctx, e),
        };
        if path.extension().and_then(|e| e.to_str()) != Some("kash") {
            return fail(
                ctx,
                FileError::new(format!(
                    "imports must be `.kash` files; `{}` is not",
                    i.path
                )),
            );
        }
        if let Some(pos) = stack.iter().position(|p| *p == path) {
            let names: Vec<String> = stack[pos..]
                .iter()
                .chain(std::iter::once(&path))
                .map(|p| {
                    p.file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                })
                .collect();
            return fail(
                ctx,
                FileError::new(format!("import cycle: {}", names.join(" -> "))),
            );
        }
        if stack.len() > MAX_IMPORT_DEPTH {
            return fail(
                ctx,
                FileError::new(format!(
                    "imports are nested more than {MAX_IMPORT_DEPTH} deep"
                )),
            );
        }
        let ns = match &i.ns {
            Some(n) => format!("{prefix}{}.", n.name),
            None => prefix.to_string(),
        };
        if !self.loaded.insert((path.clone(), ns.clone())) {
            return;
        }
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                return fail(
                    ctx,
                    FileError::new(format!("cannot read `{}`: {e}", i.path)),
                )
            }
        };
        let (lib_file, mut inner) = kashika_syntax::parse(&src);
        let outer = std::mem::take(&mut ctx.diags);
        let outer_nested = std::mem::take(&mut ctx.nested);
        for stmt in &lib_file.stmts {
            match stmt {
                Stmt::Shape(_) | Stmt::Import(_) => {}
                Stmt::Style(s) => self.style(s, &ns, ctx),
                other => ctx.diags.push(Diagnostic::error(
                    "parse",
                    other.span(),
                    format!(
                        "`{}` is not allowed in a library; library files may contain only `shape`, `style` and `import`",
                        keyword(other)
                    ),
                )),
            }
        }
        let lib_dir = path
            .parent()
            .map_or_else(|| dir.to_path_buf(), Path::to_path_buf);
        stack.push(path);
        self.defs(&lib_file.stmts, &lib_dir, &ns, stack, ctx);
        stack.pop();
        inner.append(&mut ctx.diags);
        let mut found: Vec<Nested> = inner
            .into_iter()
            .map(|d| Nested {
                at: d.span,
                hops: Vec::new(),
                diag: d,
            })
            .chain(std::mem::take(&mut ctx.nested))
            .collect();
        found.sort_by_key(|n| (n.at.line, n.at.col, n.diag.severity, n.diag.code));
        ctx.diags = outer;
        ctx.nested = outer_nested;
        for mut n in found {
            n.hops.insert(0, (i.path.clone(), n.at.line, n.at.col));
            n.at = i.path_span;
            ctx.nested.push(n);
        }
    }

    fn style(&mut self, s: &ast::StyleDef, ns: &str, ctx: &mut Ctx) {
        let a = Attrs::new(&s.attrs, "style", STYLE_KEYS, &mut ctx.diags);
        let props = attrs::style_props(&a, &mut ctx.diags);
        let name = format!("{ns}{}", s.name);
        if self.styles.iter().any(|(n, _)| *n == name) {
            ctx.diags.push(Diagnostic::error(
                "duplicate-style",
                s.span,
                format!("style `{name}` is defined twice"),
            ));
            return;
        }
        self.styles.push((name, props));
    }

    fn shape(&mut self, s: &ast::ShapeDef, dir: &Path, prefix: &str, ctx: &mut Ctx) {
        if BUILTIN_SHAPES.contains(&s.name.name.as_str()) {
            ctx.diags.push(
                Diagnostic::error(
                    "duplicate-style",
                    s.name.span,
                    format!("`{}` is a built-in shape", s.name.name),
                )
                .with_help("pick another name"),
            );
            return;
        }
        let name = format!("{prefix}{}", s.name.name);
        if self.shapes.contains_key(&name) {
            ctx.diags.push(Diagnostic::error(
                "duplicate-style",
                s.name.span,
                format!("shape `{name}` is defined twice"),
            ));
            return;
        }
        let def = match &s.body {
            ShapeBody::Paths {
                path,
                outline,
                text_area,
            } => unit_shape(path, *outline, *text_area, &mut ctx.diags).map(LibShape::Unit),
            ShapeBody::From {
                file,
                file_span,
                attrs: list,
            } => svg_shape(file, *file_span, list, dir, ctx).map(LibShape::Svg),
        };
        if let Some(d) = def {
            self.shapes.insert(name, d);
        }
    }
}

fn finish(ctx: &mut Ctx) {
    for n in std::mem::take(&mut ctx.nested) {
        ctx.diags.push(n.finish());
    }
}

fn keyword(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::Diagram(_) => "diagram",
        Stmt::Style(_) => "style",
        Stmt::Box(_) => "box",
        Stmt::Group(_) => "group",
        Stmt::Junction(_) => "junction",
        Stmt::Text(_) => "text",
        Stmt::Line(_) => "line",
        Stmt::Import(_) => "import",
        Stmt::Shape(_) => "shape",
        Stmt::Image(_) => "image",
    }
}

/// Whether `file` is a library: no `diagram`, and only `shape`, `style` and `import`.
pub(crate) fn is_library(file: &ast::File) -> bool {
    !file.stmts.is_empty()
        && file
            .stmts
            .iter()
            .all(|s| matches!(s, Stmt::Shape(_) | Stmt::Style(_) | Stmt::Import(_)))
}

/// Reports imports that follow any statement other than `diagram` or another `import`.
fn check_import_order(file: &ast::File, diags: &mut Vec<Diagnostic>) {
    let mut other_seen = false;
    for stmt in &file.stmts {
        match stmt {
            Stmt::Diagram(_) => {}
            Stmt::Import(i) if other_seen => diags.push(
                Diagnostic::error("parse", i.span, "`import` must come right after `diagram`")
                    .with_help(
                        "move every import to the top of the file, below the `diagram` line",
                    ),
            ),
            Stmt::Import(_) => {}
            _ => other_seen = true,
        }
    }
}

fn svg_shape(
    file: &str,
    span: Span,
    list: &[ast::Attr],
    dir: &Path,
    ctx: &mut Ctx,
) -> Option<SvgShape> {
    let a = Attrs::new(list, "shape", &["outline", "text-area"], &mut ctx.diags);
    let hull = a
        .get("outline")
        .and_then(|x| attrs::word(x, &["bbox", "hull"], &mut ctx.diags))
        == Some("hull");
    let below = a
        .get("text-area")
        .and_then(|x| attrs::word(x, &["below", "inside"], &mut ctx.diags))
        == Some("below");
    match ctx
        .store
        .load_svg(ctx.loader, dir, file, "`shape ... from`")
    {
        Ok(asset) => Some(SvgShape { asset, hull, below }),
        Err(e) => {
            push_file_error(&mut ctx.diags, span, e);
            None
        }
    }
}

pub(crate) fn push_file_error(diags: &mut Vec<Diagnostic>, span: Span, e: FileError) {
    let mut d = Diagnostic::error("file", span, e.message);
    if let Some(h) = e.help {
        d = d.with_help(h);
    }
    diags.push(d);
}

fn unit_shape(
    path: &(String, Span),
    outline: Option<(f64, Span)>,
    text_area: Option<([f64; 4], Span)>,
    diags: &mut Vec<Diagnostic>,
) -> Option<UnitShape> {
    let subpaths = match parse_unit_path(&path.0) {
        Ok(s) => s,
        Err(m) => {
            diags.push(Diagnostic::error("parse", path.1, m));
            return None;
        }
    };
    let outline = match outline {
        None => 0,
        Some((n, span)) => {
            if n.fract() != 0.0 || n < 1.0 || n as usize > subpaths.len() {
                diags.push(Diagnostic::error(
                    "parse",
                    span,
                    format!(
                        "`outline {n}` names no subpath; the path has {}",
                        subpaths.len()
                    ),
                ));
                return None;
            }
            n as usize - 1
        }
    };
    if subpaths[outline].is_empty() {
        diags.push(Diagnostic::error(
            "parse",
            path.1,
            "the outline subpath has no segments",
        ));
        return None;
    }
    let text_area = match text_area {
        None => Rect::new(0.0, 0.0, 1.0, 1.0),
        Some(([x, y, w, h], span)) => {
            let inside = w > 0.0
                && h > 0.0
                && x >= 0.0
                && y >= 0.0
                && x + w <= 1.0 + 1e-9
                && y + h <= 1.0 + 1e-9;
            if !inside {
                diags.push(Diagnostic::error(
                    "parse",
                    span,
                    "`text-area` must be a unit rect (x,y,w,h) inside 0..1 with a positive size",
                ));
                return None;
            }
            Rect::new(x, y, w, h)
        }
    };
    Some(UnitShape {
        subpaths,
        outline,
        text_area,
    })
}
