use std::collections::{HashMap, HashSet};

use kashika_diag::{Diagnostic, Span};
use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self, Stmt, Value};

use crate::assets::Store;
use crate::attrs::{self, Attrs, Min};
use crate::library::{self, Ctx, Library};
use crate::loader::{FileError, Loader, Options};
use crate::markup;
use crate::scene::*;
use crate::style::{self, Style, StyleProps};
use crate::textlayout::{self, ShapeTextProps};
use crate::version;

mod build;
mod canvas;
mod checks;
mod lines;
mod placement;
mod props;
mod record;

const NODE_KEYS: &[&str] = &[
    "shape",
    "style",
    "pin",
    "allow",
    "reason",
    "icon",
    "icon-pos",
    "icon-size",
    "layer",
];
const IMAGE_KEYS: &[&str] = &["pin", "fit", "layer", "allow", "reason"];
const LINE_KEYS: &[&str] = &[
    "start",
    "end",
    "stroke",
    "stroke-width",
    "stroke-style",
    "style",
    "radius",
    "label",
    "label-side",
    "ortho",
    "allow",
    "reason",
];
const TEXT_KEYS: &[&str] = &["font-size", "text-align", "color", "muted", "size", "align"];
const DIAGRAM_KEYS: &[&str] = &[
    "kashika",
    "grid",
    "clearance",
    "background",
    "icons",
    "allow",
    "margin",
];
/// Room a fitted canvas leaves to the right of and below the drawing, unless `margin=` sets it.
const DEFAULT_MARGIN: f64 = 20.0;

const JUNCTION_KEYS: &[&str] = &["dot"];

/// The keys a `box` (or, with `group`, a `group`) takes.
fn node_keys(group: bool) -> Vec<&'static str> {
    NODE_KEYS
        .iter()
        .filter(|k| !group || !matches!(**k, "shape" | "layer"))
        .chain(style::STYLE_KEYS)
        .chain(textlayout::SHAPE_TEXT_KEYS)
        .copied()
        .collect()
}

/// Every attribute key, by the statement that takes it: `diagram`, `style`, `box`, `group`,
/// `junction`, `text`, `line`, `image`, `shape from` and a text block's `text`.
pub fn attribute_keys() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("diagram", DIAGRAM_KEYS.to_vec()),
        ("style", style::def_keys()),
        ("box", node_keys(false)),
        ("group", node_keys(true)),
        ("junction", JUNCTION_KEYS.to_vec()),
        ("text", TEXT_KEYS.to_vec()),
        ("line", LINE_KEYS.to_vec()),
        ("image", IMAGE_KEYS.to_vec()),
        ("shape from", library::SHAPE_FROM_KEYS.to_vec()),
        ("block text", textlayout::BLOCK_ITEM_KEYS.to_vec()),
    ]
}

/// Longest chain of placement references, counting group frames.
const MAX_PLACEMENT_DEPTH: usize = 256;
/// Cycles longer than this print only their first few members.
const CYCLE_SHOWN: usize = 6;

#[derive(Clone, Copy)]
enum Item<'a> {
    Box(&'a ast::Node),
    Group(&'a ast::Group),
    Image(&'a ast::Image),
    Junction(&'a ast::Junction),
    Text(&'a ast::Text),
    Line(&'a ast::Line),
}

impl<'a> Item<'a> {
    fn id(self) -> Option<&'a ast::Ident> {
        match self {
            Item::Box(n) => Some(&n.id),
            Item::Group(g) => Some(&g.node.id),
            Item::Image(i) => Some(&i.id),
            Item::Junction(j) => Some(&j.id),
            Item::Text(t) => t.id.as_ref(),
            Item::Line(l) => l.id.as_ref(),
        }
    }

    fn span(self) -> Span {
        match self {
            Item::Box(n) => n.span,
            Item::Group(g) => g.node.span,
            Item::Image(i) => i.span,
            Item::Junction(j) => j.span,
            Item::Text(t) => t.span,
            Item::Line(l) => l.span,
        }
    }

    fn at_span(self) -> Span {
        match self {
            Item::Box(n) => n.at.span(),
            Item::Group(g) => g.node.at.span(),
            Item::Image(i) => i.at.span(),
            Item::Junction(j) => j.at.span(),
            Item::Text(t) => t.at.span(),
            Item::Line(l) => l.span,
        }
    }
}

struct Entry<'a> {
    item: Item<'a>,
    parent: Option<usize>,
    depth: usize,
}

struct NodeProps {
    outline: Outline,
    /// The inline `shape=`, else the `style=` style's; `None` for groups and boxes without one.
    shape: Option<String>,
    style: Style,
    pin: (&'static str, Option<f64>),
    /// The resolved size; `None` when it is invalid or can't be fitted. A group's fitted
    /// axes hold 0 until it is placed.
    size: Option<(f64, f64)>,
    /// The axes of a group's size fitted to its children.
    fitted: (bool, bool),
    text: ShapeTextProps,
    allow: Vec<Allow>,
    reason: Option<String>,
    icon: Option<IconProps>,
    image: Option<ImageProps>,
    layer: Layer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IconPos {
    Left,
    Top,
}

#[derive(Clone, Copy)]
struct ImageProps {
    asset: Option<usize>,
    fit: Fit,
}

#[derive(Clone, Copy)]
struct IconProps {
    asset: usize,
    pos: IconPos,
    size: f64,
}

struct LineProps {
    start: Marker,
    end: Marker,
    style: Style,
    labels: Vec<Label>,
    label_side: LabelSide,
    label_side_span: Span,
    ortho: bool,
    allow: Vec<Allow>,
    reason: Option<String>,
    radius: f64,
}

struct TextProps {
    content: String,
    size: f64,
    color: String,
    align: Align,
}

pub fn resolve(file: &ast::File) -> (Scene, Vec<Diagnostic>) {
    resolve_with(file, &Options::default())
}

pub fn resolve_with(file: &ast::File, opts: &Options) -> (Scene, Vec<Diagnostic>) {
    let mut r = Resolver {
        loader: Loader::new(opts),
        store: Store::default(),
        lib: Library::default(),
        entries: Vec::new(),
        ids: HashMap::new(),
        user_styles: HashMap::new(),
        nodes: HashMap::new(),
        lines: HashMap::new(),
        texts: HashMap::new(),
        dots: HashMap::new(),
        placed: Vec::new(),
        frames: HashMap::new(),
        built: HashMap::new(),
        on_stack: HashSet::new(),
        skipped: HashSet::new(),
        skipped_ids: file.failed_ids.iter().map(|i| i.name.clone()).collect(),
        edges: Vec::new(),
        literals: Vec::new(),
        grid: None,
        canvas_fit: (false, false),
        margin: DEFAULT_MARGIN,
        diags: Vec::new(),
    };
    r.icons_dir(file);
    let mut ctx = Ctx {
        loader: &r.loader,
        store: &mut r.store,
        diags: Vec::new(),
        nested: Vec::new(),
    };
    r.lib = Library::load(file, &mut ctx);
    r.diags.append(&mut ctx.diags);
    r.styles(file);
    r.collect(&file.stmts, None, 0);
    r.placed = vec![None; r.entries.len()];
    let mut scene = r.diagram(file);
    r.grid = scene.grid;
    r.validate();
    r.placement_graph();
    r.build(&mut scene);
    r.fit_canvas(&mut scene);
    scene.assets = std::mem::take(&mut r.store).into_vec();
    (scene, r.diags)
}

/// Resolves a library file checked on its own: its imports, shape files and style keys.
/// `None` when `file` is not a library (§3a), so it should be resolved as a diagram.
pub fn resolve_library_with(file: &ast::File, opts: &Options) -> Option<Vec<Diagnostic>> {
    if !library::is_library(file) {
        return None;
    }
    let loader = Loader::new(opts);
    let mut store = Store::default();
    let mut ctx = Ctx {
        loader: &loader,
        store: &mut store,
        diags: Vec::new(),
        nested: Vec::new(),
    };
    Library::load_standalone(file, &mut ctx);
    Some(ctx.diags)
}

struct Resolver<'a> {
    loader: Loader,
    store: Store,
    lib: Library,
    entries: Vec<Entry<'a>>,
    ids: HashMap<String, usize>,
    user_styles: HashMap<String, StyleProps>,
    nodes: HashMap<usize, NodeProps>,
    lines: HashMap<usize, LineProps>,
    texts: HashMap<usize, TextProps>,
    dots: HashMap<usize, bool>,
    /// Per entry: `None` until placed, then its canvas rect (junctions use a zero-size
    /// rect), or `Some(None)` when it can't be placed.
    placed: Vec<Option<Option<Rect>>>,
    /// The frame origin of each fitted group, known before its size; `None` when it can't
    /// be placed.
    frames: HashMap<usize, Option<Pt>>,
    /// Scene items built so far, by entry; `None` once `build` has taken one or it failed.
    built: HashMap<usize, Option<build::Built>>,
    /// Entries being placed; `placement_graph` keeps this chain within the depth limit.
    on_stack: HashSet<usize>,
    skipped: HashSet<usize>,
    /// Ids declared inside a skipped duplicate group or by a statement that failed to parse;
    /// references to them stay silent.
    skipped_ids: HashSet<String>,
    edges: Vec<PlacementEdge>,
    literals: Vec<Literal>,
    /// The diagram's `grid=`, which fitted sizes round up to.
    grid: Option<f64>,
    /// Which canvas axes are `auto`, fitted to what is drawn.
    canvas_fit: (bool, bool),
    /// The diagram's `margin=`, added after the drawing on a fitted axis.
    margin: f64,
    diags: Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
    /// Adds the first `diagram` statement's `icons=` directory to the search path.
    fn icons_dir(&mut self, file: &ast::File) {
        let Some(Stmt::Diagram(d)) = file.stmts.iter().find(|s| matches!(s, Stmt::Diagram(_)))
        else {
            return;
        };
        let Some(x) = d.attrs.iter().find(|a| a.key.name == "icons") else {
            return;
        };
        let ast::Value::Str(dir) = &x.value else {
            self.diags
                .push(attrs::type_err(x, "a directory path string"));
            return;
        };
        if let Err(e) = self.loader.add_icons_dir(dir) {
            self.file_error(x.span, e);
        }
    }

    fn file_error(&mut self, span: Span, e: FileError) {
        library::push_file_error(&mut self.diags, span, e);
    }

    fn diagram(&mut self, file: &ast::File) -> Scene {
        let mut scene = Scene {
            title: String::new(),
            size: (0.0, 0.0),
            grid: None,
            clearance: 6.0,
            background: None,
            allow: Vec::new(),
            shapes: Vec::new(),
            junctions: Vec::new(),
            texts: Vec::new(),
            lines: Vec::new(),
            assets: Vec::new(),
            placement: Vec::new(),
            literals: Vec::new(),
        };
        let mut seen = false;
        for stmt in &file.stmts {
            let Stmt::Diagram(d) = stmt else { continue };
            if seen {
                self.diags.push(Diagnostic::error(
                    "diagram",
                    d.span,
                    "only one `diagram` statement is allowed",
                ));
                continue;
            }
            seen = true;
            let first = file.first_stmt.map(|f| (f.line, f.col));
            if first != Some((d.span.line, d.span.col)) {
                self.diags.push(Diagnostic::error(
                    "diagram",
                    d.span,
                    "`diagram` must be the first statement",
                ));
            }
            scene.title = markup::unescape(&d.title);
            let ast::Size { w, h } = d.size.unwrap_or(ast::Size { w: None, h: None });
            if self.size_ok((w.unwrap_or(1.0), h.unwrap_or(1.0)), d.span, "the diagram") {
                scene.size = (w.unwrap_or(0.0), h.unwrap_or(0.0));
                self.canvas_fit = (w.is_none(), h.is_none());
            }

            let a = Attrs::new(&d.attrs, "diagram", DIAGRAM_KEYS, &mut self.diags);
            if let Some(x) = a.get("kashika") {
                let shown = match &x.value {
                    Value::Str(s) => version::check(s, version::TOOL_VERSION).err(),
                    _ => Some(version::Problem {
                        message: "`kashika=` must be a quoted version string".to_string(),
                        help: version::form_help().to_string(),
                    }),
                };
                if let Some(p) = shown {
                    self.diags
                        .push(Diagnostic::error("version", x.span, p.message).with_help(p.help));
                }
            }
            if let Some(x) = a.get("grid") {
                scene.grid = attrs::num_in(x, Min::Positive, &mut self.diags);
            }
            if let Some(x) = a.get("clearance") {
                if let Some(v) = attrs::num_in(x, Min::NonNegative, &mut self.diags) {
                    scene.clearance = v;
                }
            }
            if let Some(x) = a.get("background") {
                scene.background = attrs::color(x, &mut self.diags).filter(|c| c != "none");
            }
            if let Some(x) = a.get("allow") {
                scene.allow = self.allow_list(x, false);
            }
            if let Some(x) = a.get("margin") {
                if w.is_some() && h.is_some() {
                    self.diags.push(
                        Diagnostic::error(
                            "attr",
                            x.span,
                            "`margin` applies only to a canvas that fits its content",
                        )
                        .with_help("remove `margin`, or leave out `size` (or write `size (w, auto)`) to fit the canvas"),
                    );
                } else if let Some(v) = attrs::num_in(x, Min::NonNegative, &mut self.diags) {
                    self.margin = v;
                }
            }
        }
        if !seen && !file.failed_diagram {
            let span = file.stmts.first().map_or(Span::new(1, 1, 1, 1), Stmt::span);
            self.diags.push(
                Diagnostic::error("diagram", span, "missing `diagram` statement")
                    .with_help("start the file with `diagram \"<title>\" size (w,h)`"),
            );
        }
        scene
    }

    fn styles(&mut self, file: &ast::File) {
        for (name, props) in &self.lib.styles {
            self.user_styles.insert(name.clone(), props.clone());
        }
        let mut local = HashSet::new();
        for stmt in &file.stmts {
            let Stmt::Style(s) = stmt else { continue };
            let mut props = library::style_props(s, &mut self.diags);
            library::boundary_size(&s.name, s, &mut props, &mut self.diags);
            if let Some(x) = library::style_attr(s, "shape") {
                if library::style_shape(&s.name, x, &mut self.diags) {
                    props.shape = self.shape_attr(x);
                }
            }
            if self.user_styles.insert(s.name.clone(), props).is_some() {
                let first = Some(&s.name)
                    .filter(|n| !local.contains(*n))
                    .and_then(|n| self.lib.style_origin(n));
                self.diags.push(Diagnostic::error(
                    "duplicate-style",
                    s.span,
                    library::twice("style", &s.name, first),
                ));
            }
            local.insert(s.name.clone());
        }
    }

    fn collect(&mut self, stmts: &'a [Stmt], parent: Option<usize>, depth: usize) {
        for stmt in stmts {
            let item = match stmt {
                Stmt::Box(n) => Item::Box(n),
                Stmt::Group(g) => Item::Group(g),
                Stmt::Junction(j) => Item::Junction(j),
                Stmt::Text(t) => Item::Text(t),
                Stmt::Line(l) => Item::Line(l),
                Stmt::Diagram(_) => {
                    if parent.is_some() {
                        self.diags.push(Diagnostic::error(
                            "diagram",
                            stmt.span(),
                            "`diagram` is only allowed as the first top-level statement",
                        ));
                    }
                    continue;
                }
                Stmt::Image(i) => {
                    if parent.is_some() {
                        self.diags.push(Diagnostic::error(
                            "parse",
                            stmt.span(),
                            "`image` is only allowed at the top level",
                        ));
                        continue;
                    }
                    Item::Image(i)
                }
                Stmt::Import(_) => {
                    self.top_level_only("import", stmt.span(), parent);
                    continue;
                }
                Stmt::Shape(_) => {
                    self.top_level_only("shape", stmt.span(), parent);
                    continue;
                }
                Stmt::Style(_) => {
                    self.top_level_only("style", stmt.span(), parent);
                    continue;
                }
            };
            let idx = self.entries.len();
            self.entries.push(Entry {
                item,
                parent,
                depth,
            });
            if parent.is_some_and(|p| self.skipped.contains(&p)) {
                self.skipped.insert(idx);
                if let Some(id) = item.id() {
                    self.skipped_ids.insert(id.name.clone());
                }
            } else if let Some(id) = item.id() {
                self.register(id, idx);
            }
            if let Stmt::Group(g) = stmt {
                self.collect(&g.children, Some(idx), depth + 1);
            }
        }
    }

    fn top_level_only(&mut self, kw: &str, span: Span, parent: Option<usize>) {
        if parent.is_some() {
            self.diags.push(Diagnostic::error(
                "parse",
                span,
                format!("`{kw}` is only allowed at the top level"),
            ));
        }
    }

    fn register(&mut self, id: &ast::Ident, idx: usize) {
        if let Some(&prev) = self.ids.get(&id.name) {
            self.skipped.insert(idx);

            let line = self.entries[prev].item.span().line;
            self.diags.push(
                Diagnostic::error(
                    "duplicate-id",
                    id.span,
                    format!("`{}` is already defined on line {line}", id.name),
                )
                .with_help("ids are unique across the whole file"),
            );
        } else {
            self.ids.insert(id.name.clone(), idx);
        }
    }
}
