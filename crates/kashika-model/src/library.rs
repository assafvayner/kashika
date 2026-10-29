use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use kashika_diag::{Diagnostic, Span, did_you_mean};
use kashika_geom::{Rect, parse_unit_path};
use kashika_syntax::ast::{self, ShapeBody, Stmt};

use crate::assets::Store;
use crate::attrs::{self, Attrs};
use crate::loader::{FileError, Found, Loader};
use crate::shapes::{SHAPES, UnitShape, builtin_shape, shape_names};
use crate::style::{self, StyleProps};

/// Deepest chain of nested imports followed before giving up.
const MAX_IMPORT_DEPTH: usize = 32;
/// Most library instances (a file under one namespace) registered for one diagram.
const MAX_INSTANCES: usize = 1000;
/// Largest library file read.
const MAX_IMPORT_BYTES: u64 = 1024 * 1024;
/// Keys a `shape <name> from "<file.svg>"` takes.
pub(crate) const SHAPE_FROM_KEYS: &[&str] = &["outline", "text-area"];
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
    /// (canonical path, namespace prefix) of every library instance registered so far.
    loaded: HashSet<(PathBuf, String)>,
    /// Every library file read so far, parsed once whatever its namespaces.
    modules: HashMap<PathBuf, Result<Rc<Module>, FileError>>,
    /// Shape bodies of library files by (path, statement index), resolved on first use.
    bodies: HashMap<(PathBuf, usize), (Option<LibShape>, Vec<Diagnostic>)>,
    /// Library instances registered so far, against `MAX_INSTANCES`.
    instances: usize,
    /// Where each imported style was defined, by qualified name.
    style_origins: HashMap<String, Origin>,
    /// Where each imported shape was defined, by qualified name.
    shape_origins: HashMap<String, Origin>,
}

/// The library file a definition came from and the main file's import that brought it in.
#[derive(Debug, Clone)]
pub(crate) struct Origin {
    /// The library's path as its importer wrote it.
    file: String,
    /// The path string of the main file's `import` that leads to the library.
    import: Span,
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "in `{}` (imported at {}:{})",
            self.file, self.import.line, self.import.col
        )
    }
}

/// A library file, read, parsed and partly resolved once.
struct Module {
    path: PathBuf,
    dir: PathBuf,
    file: ast::File,
    parse_diags: Vec<Diagnostic>,
    /// Per `style` statement index: its properties and the diagnostics they raised.
    styles: HashMap<usize, (StyleProps, Vec<Diagnostic>)>,
    /// Per `import` statement index: the file it names.
    imports: HashMap<usize, Result<PathBuf, FileError>>,
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

/// Where an import is in the walk: the files above it and how many of them are libraries.
struct Chain {
    stack: Vec<PathBuf>,
    depth: usize,
    /// The path string of the main file's `import` being followed.
    top: Span,
}

impl Library {
    pub fn load(file: &ast::File, ctx: &mut Ctx) -> Library {
        let mut lib = Library::default();
        check_import_order(file, &mut ctx.diags);
        lib.main_defs(file, ctx);
        lib
    }

    /// Loads a library file checked on its own, as if it were imported without `as`.
    pub fn load_standalone(file: &ast::File, ctx: &mut Ctx) -> Library {
        let mut lib = Library::default();
        let mut pending = Vec::new();
        for stmt in &file.stmts {
            if let Stmt::Style(s) = stmt {
                let props = style_props(s, &mut ctx.diags);
                if let Some(k) = lib.add_style(s, "", props, None, ctx) {
                    pending.push((k, s));
                }
            }
        }
        lib.main_defs(file, ctx);
        lib.style_shapes(&pending, "", &mut ctx.diags);
        lib
    }

    /// The main file's shapes and imports.
    fn main_defs(&mut self, file: &ast::File, ctx: &mut Ctx) {
        let root = ctx.loader.root().to_path_buf();
        let mut chain = Chain {
            stack: ctx
                .loader
                .main()
                .map(Path::to_path_buf)
                .into_iter()
                .collect(),
            depth: 0,
            top: Span::new(1, 1, 1, 1),
        };
        for stmt in &file.stmts {
            match stmt {
                Stmt::Shape(s) => {
                    if self.name_free(s, "", ctx) {
                        if let Some(d) = shape_body(s, &root, ctx) {
                            self.shapes.insert(s.name.name.clone(), d);
                        }
                    }
                }
                Stmt::Import(i) => {
                    let path = find_import(ctx.loader, &root, i);
                    self.import(i, path, "", &mut chain, ctx);
                }
                _ => {}
            }
        }
        finish(ctx);
    }

    fn import(
        &mut self,
        i: &ast::Import,
        path: Result<PathBuf, FileError>,
        prefix: &str,
        chain: &mut Chain,
        ctx: &mut Ctx,
    ) {
        let fail = |ctx: &mut Ctx, e: FileError| push_file_error(&mut ctx.diags, i.path_span, e);
        let path = match path {
            Ok(p) => p,
            Err(e) => return fail(ctx, e),
        };
        if let Some(pos) = chain.stack.iter().position(|p| *p == path) {
            let names: Vec<String> = chain.stack[pos..]
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
        if chain.depth >= MAX_IMPORT_DEPTH {
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
        if self.loaded.contains(&(path.clone(), ns.clone())) {
            return;
        }
        if self.instances >= MAX_INSTANCES {
            if self.instances == MAX_INSTANCES {
                self.instances += 1;
                fail(
                    ctx,
                    FileError {
                        message: format!(
                            "more than {MAX_INSTANCES} library imports in total, counting each namespace a library is imported under"
                        ),
                        help: Some(
                            "import shared libraries once, instead of under several namespaces at each level"
                                .into(),
                        ),
                    },
                );
            }
            return;
        }
        self.instances += 1;
        self.loaded.insert((path.clone(), ns.clone()));
        if chain.depth == 0 {
            chain.top = i.path_span;
        }
        let origin = Origin {
            file: i.path.clone(),
            import: chain.top,
        };
        let m = match self.module(&path, &i.path, ctx.loader) {
            Ok(m) => m,
            Err(e) => return fail(ctx, e),
        };
        let mut inner = m.parse_diags.clone();
        let outer = std::mem::take(&mut ctx.diags);
        let outer_nested = std::mem::take(&mut ctx.nested);
        let mut pending = Vec::new();
        for (k, stmt) in m.file.stmts.iter().enumerate() {
            match stmt {
                Stmt::Shape(_) | Stmt::Import(_) => {}
                Stmt::Style(s) => {
                    let (props, diags) = &m.styles[&k];
                    ctx.diags.extend(diags.iter().cloned());
                    if let Some(at) = self.add_style(s, &ns, props.clone(), Some(&origin), ctx) {
                        pending.push((at, s));
                    }
                }
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
        chain.stack.push(path);
        chain.depth += 1;
        self.module_defs(&m, &ns, &origin, chain, ctx);
        chain.depth -= 1;
        chain.stack.pop();
        self.style_shapes(&pending, &ns, &mut ctx.diags);
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

    /// The parsed library at `path`, read on first use.
    fn module(
        &mut self,
        path: &Path,
        written: &str,
        loader: &Loader,
    ) -> Result<Rc<Module>, FileError> {
        if let Some(m) = self.modules.get(path) {
            return m.clone();
        }
        let m = read_module(path, written, loader).map(Rc::new);
        self.modules.insert(path.to_path_buf(), m.clone());
        m
    }

    /// A library's shapes and imports, registered under `ns`.
    fn module_defs(
        &mut self,
        m: &Module,
        ns: &str,
        origin: &Origin,
        chain: &mut Chain,
        ctx: &mut Ctx,
    ) {
        for (k, stmt) in m.file.stmts.iter().enumerate() {
            match stmt {
                Stmt::Shape(s) => {
                    if !self.name_free(s, ns, ctx) {
                        continue;
                    }
                    let key = (m.path.clone(), k);
                    if !self.bodies.contains_key(&key) {
                        let outer = std::mem::take(&mut ctx.diags);
                        let def = shape_body(s, &m.dir, ctx);
                        let diags = std::mem::replace(&mut ctx.diags, outer);
                        self.bodies.insert(key.clone(), (def, diags));
                    }
                    let (def, diags) = &self.bodies[&key];
                    ctx.diags.extend(diags.iter().cloned());
                    if let Some(d) = def.clone() {
                        let name = format!("{ns}{}", s.name.name);
                        self.shape_origins.insert(name.clone(), origin.clone());
                        self.shapes.insert(name, d);
                    }
                }
                Stmt::Import(i) => {
                    let path = m.imports[&k].clone();
                    self.import(i, path, ns, chain, ctx);
                }
                _ => {}
            }
        }
    }

    /// Registers `s` under `ns`; its index in `styles`, or `None` when the name is taken.
    fn add_style(
        &mut self,
        s: &ast::StyleDef,
        ns: &str,
        props: StyleProps,
        origin: Option<&Origin>,
        ctx: &mut Ctx,
    ) -> Option<usize> {
        let name = format!("{ns}{}", s.name);
        if self.styles.iter().any(|(n, _)| *n == name) {
            ctx.diags.push(Diagnostic::error(
                "duplicate-style",
                s.span,
                twice("style", &name, self.style_origins.get(&name)),
            ));
            return None;
        }
        if let Some(o) = origin {
            self.style_origins.insert(name.clone(), o.clone());
        }
        self.styles.push((name, props));
        Some(self.styles.len() - 1)
    }

    /// Resolves the `shape=` of each registered library style, once every shape under
    /// `ns` is known.
    fn style_shapes(
        &mut self,
        pending: &[(usize, &ast::StyleDef)],
        ns: &str,
        diags: &mut Vec<Diagnostic>,
    ) {
        for &(k, s) in pending {
            let (name, props) = &mut self.styles[k];
            boundary_size(name, s, props, diags);
            let Some(a) = style_attr(s, "shape") else {
                continue;
            };
            let shape = style_shape(&self.styles[k].0, a, diags)
                .then(|| shape_name(a, ns, &self.shapes, diags))
                .flatten();
            self.styles[k].1.shape = shape;
        }
    }

    /// Where the imported style `name` was defined.
    pub fn style_origin(&self, name: &str) -> Option<&Origin> {
        self.style_origins.get(name)
    }

    /// Whether `s` may be defined under `prefix`: not a built-in name, not defined yet.
    fn name_free(&self, s: &ast::ShapeDef, prefix: &str, ctx: &mut Ctx) -> bool {
        if builtin_shape(&s.name.name).is_some() {
            ctx.diags.push(
                Diagnostic::error(
                    "duplicate-style",
                    s.name.span,
                    format!("`{}` is a built-in shape", s.name.name),
                )
                .with_help("pick another name"),
            );
            return false;
        }
        let name = format!("{prefix}{}", s.name.name);
        if self.shapes.contains_key(&name) {
            ctx.diags.push(Diagnostic::error(
                "duplicate-style",
                s.name.span,
                twice("shape", &name, self.shape_origins.get(&name)),
            ));
            return false;
        }
        true
    }
}

/// The message for a second definition of `name`, naming where an imported first one is.
pub(crate) fn twice(kind: &str, name: &str, first: Option<&Origin>) -> String {
    match first {
        Some(o) => format!("{kind} `{name}` is already defined {o}"),
        None => format!("{kind} `{name}` is defined twice"),
    }
}

fn read_module(path: &Path, written: &str, loader: &Loader) -> Result<Module, FileError> {
    let bytes = loader.read(&Found::File(path.to_path_buf()), written, MAX_IMPORT_BYTES)?;
    let src = String::from_utf8(bytes).map_err(|_| {
        FileError::new(format!(
            "cannot read `{written}`: stream did not contain valid UTF-8"
        ))
    })?;
    let (file, parse_diags) = kashika_syntax::parse(&src);
    let dir = path.parent().unwrap_or(path).to_path_buf();
    let mut styles = HashMap::new();
    let mut imports = HashMap::new();
    for (k, stmt) in file.stmts.iter().enumerate() {
        match stmt {
            Stmt::Style(s) => {
                let mut diags = Vec::new();
                let props = style_props(s, &mut diags);
                styles.insert(k, (props, diags));
            }
            Stmt::Import(i) => {
                imports.insert(k, find_import(loader, &dir, i));
            }
            _ => {}
        }
    }
    Ok(Module {
        path: path.to_path_buf(),
        dir,
        file,
        parse_diags,
        styles,
        imports,
    })
}

/// The `.kash` file an import names, found from `dir`.
fn find_import(loader: &Loader, dir: &Path, i: &ast::Import) -> Result<PathBuf, FileError> {
    let path = match loader.find(dir, &i.path)? {
        Found::File(p) => p,
        Found::Bundled(_) => {
            return Err(FileError::new(format!(
                "`{}` is a bundled icon, not a library",
                i.path
            )));
        }
    };
    if path.extension().and_then(|e| e.to_str()) != Some("kash") {
        return Err(FileError::new(format!(
            "imports must be `.kash` files; `{}` is not",
            i.path
        )));
    }
    Ok(path)
}

/// The keys of a `style` definition, besides `shape`, which needs the shapes in scope.
pub(crate) fn style_props(s: &ast::StyleDef, diags: &mut Vec<Diagnostic>) -> StyleProps {
    let a = Attrs::new(&s.attrs, "style", &style::def_keys(), diags);
    let mut props = attrs::style_props(&a, diags);
    if let Some(x) = a.get("size") {
        props.size = attrs::size(x, diags);
    }
    props
}

/// The `key=` of a `style` definition that `Attrs` keeps: the first, without `@`.
pub(crate) fn style_attr<'s>(s: &'s ast::StyleDef, key: &str) -> Option<&'s ast::Attr> {
    s.attrs
        .iter()
        .find(|a| a.key.name == key && a.at_t.is_none())
}

/// Drops `size` from the style `name` when it is `boundary`, which every group starts from:
/// a size there would size every group.
pub(crate) fn boundary_size(
    name: &str,
    s: &ast::StyleDef,
    props: &mut StyleProps,
    diags: &mut Vec<Diagnostic>,
) {
    if name != "boundary" || props.size.is_none() {
        return;
    }
    let span = style_attr(s, "size").map_or(s.span, |a| a.span);
    diags.push(
        Diagnostic::error(
            "attr",
            span,
            "the `boundary` style can't set `size`: every group starts from it",
        )
        .with_help("drop `size` here, and set it with another style or inline on the group"),
    );
    props.size = None;
}

/// Whether the style `name` may take the `shape=` in `a`: every group draws with the
/// `boundary` style, and groups take no shape.
pub(crate) fn style_shape(name: &str, a: &ast::Attr, diags: &mut Vec<Diagnostic>) -> bool {
    if name != "boundary" {
        return true;
    }
    diags.push(
        Diagnostic::error(
            "attr",
            a.span,
            "the `boundary` style can't set `shape`: every group starts from it, and groups take no shape",
        )
        .with_help("drop `shape` here, and set it with another style or inline on the box"),
    );
    false
}

/// The qualified shape that `shape=` names when written under namespace `ns`, if it
/// exists: a built-in, else `ns` followed by the name.
pub(crate) fn shape_name(
    a: &ast::Attr,
    ns: &str,
    shapes: &BTreeMap<String, LibShape>,
    diags: &mut Vec<Diagnostic>,
) -> Option<String> {
    let ast::Value::Word(w) = &a.value else {
        diags.push(attrs::type_err(a, "a shape name"));
        return None;
    };
    if builtin_shape(w).is_some() {
        return Some(w.clone());
    }
    let name = format!("{ns}{w}");
    if shapes.contains_key(&name) {
        return Some(name);
    }
    let names = shape_names()
        .map(|n| -> &str { n })
        .chain(shapes.keys().filter_map(|k| k.strip_prefix(ns)));
    let mut d = Diagnostic::error("unknown-shape", a.span, format!("unknown shape `{w}`"));
    let lower = w.to_ascii_lowercase();
    let keyword = SHAPES
        .iter()
        .find(|s| s.name == lower || s.keywords.contains(&lower.as_str()))
        .map(|s| s.name);
    if let Some(s) = keyword.or_else(|| did_you_mean(w, names)) {
        d = d.with_help(format!("did you mean `{s}`?"));
    }
    if crate::icons::bundled(w).is_some() {
        d = d.with_help(format!(
            "`{w}` is a bundled icon: put it on a box with `icon=\"{w}\"`, or draw the box as it with `shape {w} from \"{w}\"`"
        ));
    }
    let list: Vec<String> = shape_names().map(|n| format!("`{n}`")).collect();
    d = d.with_help(format!("built-in shapes: {}", list.join(", ")));
    diags.push(d);
    None
}

fn shape_body(s: &ast::ShapeDef, dir: &Path, ctx: &mut Ctx) -> Option<LibShape> {
    match &s.body {
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
    !file.failed_diagram
        && !file.stmts.is_empty()
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
    let a = Attrs::new(list, "shape", SHAPE_FROM_KEYS, &mut ctx.diags);
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
        hidden_outline: false,
        text_area,
    })
}
