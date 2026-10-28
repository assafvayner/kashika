use std::collections::{HashMap, HashSet};

use kashika_diag::{did_you_mean, Diagnostic, Span, LINT_CODES};
use kashika_geom::{point_at, round_corners_tagged, PathSeg, Pt, Rect, TOLERANCE};
use kashika_syntax::ast::{self, Axis, Coord, Point, SegKind, Side, Stmt, Target, Value};

use crate::anchor;
use crate::assets::Store;
use crate::attrs::{self, Attrs, Min, LIMIT};
use crate::form::{self, Form};
use crate::library::{self, Ctx, LibShape, Library, SvgShape};
use crate::loader::{FileError, Loader, Options};
use crate::markup;
use crate::scene::*;
use crate::shapes::{self, BUILTIN_SHAPES};
use crate::style::{self, Style, StyleProps, STYLE_KEYS};
use crate::textlayout::{self, ShapeInput, ShapeTextProps, SHAPE_TEXT_KEYS};
use crate::version;

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
];
const IMAGE_KEYS: &[&str] = &["pin", "fit", "allow", "reason"];
const LINE_KEYS: &[&str] = &[
    "start",
    "end",
    "stroke",
    "stroke-width",
    "stroke-style",
    "style",
    "radius",
    "label",
    "ortho",
    "allow",
    "reason",
];
const TEXT_KEYS: &[&str] = &["size", "color", "align", "muted"];
const DIAGRAM_KEYS: &[&str] = &[
    "kashika",
    "grid",
    "clearance",
    "background",
    "icons",
    "allow",
];
const MARKERS: &[&str] = &["none", "arrow", "open", "diamond", "dot"];
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
    /// The `shape=` name; `None` for groups and boxes without one.
    shape: Option<String>,
    style: Style,
    pin: (&'static str, Option<f64>),
    size_ok: bool,
    text: ShapeTextProps,
    allow: Vec<Allow>,
    reason: Option<String>,
    icon: Option<IconProps>,
    image: Option<ImageProps>,
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
        placed: HashMap::new(),
        on_stack: HashSet::new(),
        skipped: HashSet::new(),
        skipped_ids: HashSet::new(),
        edges: Vec::new(),
        literals: Vec::new(),
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
    let mut scene = r.diagram(file);
    r.validate();
    r.placement_graph();
    r.build(&mut scene);
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
    /// Canvas rect per placed entry (junctions use a zero-size rect); `None` = failed.
    placed: HashMap<usize, Option<Rect>>,
    /// Entries being placed; `placement_graph` keeps this chain within the depth limit.
    on_stack: HashSet<usize>,
    skipped: HashSet<usize>,
    /// Ids declared inside a skipped duplicate group; references to them stay silent.
    skipped_ids: HashSet<String>,
    edges: Vec<PlacementEdge>,
    literals: Vec<Literal>,
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
            if self.size_ok(d.size, d.span, "the diagram") {
                scene.size = d.size;
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
        }
        if !seen {
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
        for stmt in &file.stmts {
            let Stmt::Style(s) = stmt else { continue };
            let a = Attrs::new(&s.attrs, "style", STYLE_KEYS, &mut self.diags);
            let props = attrs::style_props(&a, &mut self.diags);
            if self.user_styles.insert(s.name.clone(), props).is_some() {
                self.diags.push(Diagnostic::error(
                    "duplicate-style",
                    s.span,
                    format!("style `{}` is defined twice", s.name),
                ));
            }
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

    fn allow_list(&mut self, a: &ast::Attr, check_ids: bool) -> Vec<Allow> {
        let ast::Value::Word(w) = &a.value else {
            self.diags.push(attrs::type_err(
                a,
                "lint codes like `cross` or `cross:other_line`",
            ));
            return Vec::new();
        };
        let mut out = Vec::new();
        for item in w.split(',') {
            let (code, other) = match item.split_once(':') {
                Some((c, o)) => (c, Some(o.to_string())),
                None => (item, None),
            };
            if !LINT_CODES.contains(&code) {
                let mut d = Diagnostic::error("attr", a.span, format!("unknown lint `{code}`"));
                if let Some(s) = did_you_mean(code, LINT_CODES.iter().copied()) {
                    d = d.with_help(format!("did you mean `{s}`?"));
                }
                self.diags.push(d);
                continue;
            }
            if let Some(o) = &other {
                if !check_ids {
                    self.diags.push(Diagnostic::error(
                        "attr",
                        a.span,
                        "diagram-level `allow` cannot name another line",
                    ));
                    continue;
                }
                if !self.ids.contains_key(o) {
                    if self.skipped_ids.contains(o) {
                        continue;
                    }
                    let d = self.unknown_id(o, a.span);
                    self.diags.push(d);
                    continue;
                }
            }
            out.push(Allow {
                code: code.to_string(),
                other,
            });
        }
        out
    }

    /// Checks that every value is a finite number within `LIMIT` of zero.
    fn finite(&mut self, vals: &[f64], span: Span) -> bool {
        if vals.iter().any(|v| !v.is_finite()) {
            self.diags.push(Diagnostic::error(
                "degenerate-curve",
                span,
                "value is not a finite number",
            ));
            false
        } else if vals.iter().any(|v| v.abs() > LIMIT) {
            self.diags.push(
                Diagnostic::error(
                    "degenerate-curve",
                    span,
                    format!("value is out of range (more than {LIMIT} from the origin)"),
                )
                .with_help(format!("keep coordinates within -{LIMIT}..{LIMIT}")),
            );
            false
        } else {
            true
        }
    }

    fn size_ok(&mut self, (w, h): (f64, f64), span: Span, what: &str) -> bool {
        if !self.finite_only(&[w, h], span) {
            return false;
        }
        let msg = if w <= 0.0 || h <= 0.0 {
            format!("{what} must have a positive size")
        } else if w > LIMIT || h > LIMIT {
            format!("{what} is larger than {LIMIT}")
        } else {
            return true;
        };
        self.diags.push(Diagnostic::error("size", span, msg));
        false
    }

    fn finite_only(&mut self, vals: &[f64], span: Span) -> bool {
        if vals.iter().all(|v| v.is_finite()) {
            return true;
        }
        self.finite(vals, span)
    }

    fn unknown_id(&self, name: &str, span: Span) -> Diagnostic {
        let mut d = Diagnostic::error("unknown-id", span, format!("unknown id `{name}`"));
        if let Some(s) = did_you_mean(name, self.ids.keys().map(String::as_str)) {
            d = d.with_help(format!("did you mean `{s}`?"));
        }
        d
    }

    fn validate(&mut self) {
        for idx in 0..self.entries.len() {
            match self.entries[idx].item {
                Item::Box(n) => {
                    let p = self.node_props(n, false);
                    self.nodes.insert(idx, p);
                }
                Item::Group(g) => {
                    let p = self.node_props(&g.node, true);
                    self.nodes.insert(idx, p);
                }
                Item::Image(i) => {
                    let p = self.image_props(i);
                    self.nodes.insert(idx, p);
                }
                Item::Junction(j) => {
                    let a = Attrs::new(&j.attrs, "junction", &["dot"], &mut self.diags);
                    let dot = a
                        .get("dot")
                        .is_some_and(|x| attrs::flag(x, &mut self.diags));
                    self.dots.insert(idx, dot);
                }
                Item::Text(t) => {
                    let p = self.text_props(t);
                    self.texts.insert(idx, p);
                }
                Item::Line(l) => {
                    let p = self.line_props(l);
                    self.lines.insert(idx, p);
                }
            }
        }
    }

    fn node_props(&mut self, n: &ast::Node, group: bool) -> NodeProps {
        let what = if group { "group" } else { "box" };
        let keys: Vec<&str> = NODE_KEYS
            .iter()
            .filter(|k| !group || **k != "shape")
            .chain(STYLE_KEYS)
            .chain(SHAPE_TEXT_KEYS)
            .copied()
            .collect();
        let a = Attrs::new(&n.attrs, what, &keys, &mut self.diags);

        let size_ok = self.size_ok(n.size, n.span, &format!("`{}`", n.id.name));
        let shape = a.get("shape").and_then(|x| self.shape_name(x));
        let mut props = StyleProps::default();
        if group {
            let user = self
                .user_styles
                .get("boundary")
                .cloned()
                .unwrap_or_default();
            props = props
                .overlay(&style::builtin("boundary").unwrap_or_default())
                .overlay(&user);
        }
        if let Some(x) = a.get("style") {
            if let Some(named) = self.named_style(x) {
                props = props.overlay(&named);
            }
        }
        props = props.overlay(&attrs::style_props(&a, &mut self.diags));
        let rounded = shape.as_deref() == Some("rounded");
        if rounded && props.radius.is_none() {
            props.radius = Some(8.0);
        }
        let outline = match shape.as_deref() {
            Some(s) if s != "rect" && s != "rounded" => Outline::Path,
            _ if rounded || props.radius.unwrap_or(0.0) > 0.0 => Outline::Rounded,
            _ => Outline::Rect,
        };
        let pin = a
            .get("pin")
            .and_then(|x| self.pin(x))
            .unwrap_or(("tl", None));
        let text = textlayout::shape_text_props(&a, group, &mut self.diags);
        let allow = a
            .get("allow")
            .map(|x| self.allow_list(x, true))
            .unwrap_or_default();
        let reason = a
            .get("reason")
            .and_then(|x| attrs::text(x, &mut self.diags))
            .map(|r| markup::unescape(&r));
        let style = props.resolve();
        let icon = self.icon_props(&a, group, style.font_size);
        NodeProps {
            outline,
            shape,
            style,
            pin,
            size_ok,
            text,
            allow,
            reason,
            icon,
            image: None,
        }
    }

    fn icon_props(&mut self, a: &Attrs<'_>, group: bool, font_size: f64) -> Option<IconProps> {
        let pos = match a.get("icon-pos") {
            Some(x) => match attrs::word(x, &["left", "top"], &mut self.diags) {
                Some("top") if group => {
                    self.diags.push(Diagnostic::error(
                        "attr",
                        x.span,
                        "groups only support `icon-pos=left`",
                    ));
                    IconPos::Left
                }
                Some("top") => IconPos::Top,
                _ => IconPos::Left,
            },
            None => IconPos::Left,
        };
        let size = a.get("icon-size").and_then(|x| {
            let v = attrs::num(x, &mut self.diags)?;
            if v <= 0.0 {
                self.diags.push(Diagnostic::error(
                    "size",
                    x.span,
                    "`icon-size` must be positive",
                ));
                return None;
            }
            Some(v)
        });
        let size = size.unwrap_or(if group { font_size * 1.25 } else { 24.0 });
        let Some(x) = a.get("icon") else {
            for k in ["icon-pos", "icon-size"] {
                if let Some(x) = a.get(k) {
                    self.diags.push(Diagnostic::error(
                        "attr",
                        x.span,
                        format!("`{k}` needs `icon=`"),
                    ));
                }
            }
            return None;
        };
        let name = attrs::text(x, &mut self.diags)?;
        let asset = self.load_asset(&name, x.span)?;
        Some(IconProps { asset, pos, size })
    }

    fn image_props(&mut self, i: &ast::Image) -> NodeProps {
        let a = Attrs::new(&i.attrs, "image", IMAGE_KEYS, &mut self.diags);
        let size_ok = self.size_ok(i.size, i.span, &format!("`{}`", i.id.name));
        let pin = a
            .get("pin")
            .and_then(|x| self.pin(x))
            .unwrap_or(("tl", None));
        let fit = match a
            .get("fit")
            .and_then(|x| attrs::word(x, &["contain", "cover", "stretch"], &mut self.diags))
        {
            Some("cover") => Fit::Cover,
            Some("stretch") => Fit::Stretch,
            _ => Fit::Contain,
        };
        let allow = a
            .get("allow")
            .map(|x| self.allow_list(x, true))
            .unwrap_or_default();
        let reason = a
            .get("reason")
            .and_then(|x| attrs::text(x, &mut self.diags))
            .map(|r| markup::unescape(&r));
        let asset = self.load_asset(&i.file, i.file_span);
        NodeProps {
            outline: Outline::Rect,
            shape: None,
            style: Style::default(),
            pin,
            size_ok,
            text: textlayout::shape_text_props(
                &Attrs::new(&[], "image", &[], &mut Vec::new()),
                false,
                &mut Vec::new(),
            ),
            allow,
            reason,
            icon: None,
            image: Some(ImageProps { asset, fit }),
        }
    }

    fn load_asset(&mut self, name: &str, span: Span) -> Option<usize> {
        let root = self.loader.root().to_path_buf();
        match self.store.load(&self.loader, &root, name) {
            Ok(i) => Some(i),
            Err(e) => {
                self.file_error(span, e);
                None
            }
        }
    }

    fn shape_name(&mut self, a: &ast::Attr) -> Option<String> {
        let ast::Value::Word(w) = &a.value else {
            self.diags.push(attrs::type_err(a, "a shape name"));
            return None;
        };
        if BUILTIN_SHAPES.contains(&w.as_str()) || self.lib.shapes.contains_key(w) {
            return Some(w.clone());
        }
        let names = BUILTIN_SHAPES
            .iter()
            .copied()
            .chain(self.lib.shapes.keys().map(String::as_str));
        let mut d = Diagnostic::error("unknown-shape", a.span, format!("unknown shape `{w}`"));
        if let Some(s) = did_you_mean(w, names) {
            d = d.with_help(format!("did you mean `{s}`?"));
        }
        self.diags.push(d);
        None
    }

    /// The outline and drawing of a shape name on `rect`: a custom definition, else a
    /// built-in sized for `rect`. `None` for `rect` and `rounded`.
    fn form(&self, name: &str, rect: Rect) -> Option<Form> {
        match self.lib.shapes.get(name) {
            Some(LibShape::Unit(u)) => Some(Form::from_unit(u, rect)),
            Some(LibShape::Svg(s)) => Some(Form::from_svg(self.store.get(s.asset), s.hull, rect)),
            None => shapes::builtin(name, rect.w, rect.h).map(|u| Form::from_unit(&u, rect)),
        }
    }

    fn svg_shape(&self, name: &str) -> Option<SvgShape> {
        match self.lib.shapes.get(name) {
            Some(LibShape::Svg(s)) => Some(*s),
            _ => None,
        }
    }

    fn named_style(&mut self, a: &ast::Attr) -> Option<StyleProps> {
        let ast::Value::Word(name) = &a.value else {
            self.diags.push(attrs::type_err(a, "a style name"));
            return None;
        };
        let builtin = style::builtin(name);
        let user = self.user_styles.get(name).cloned();
        if builtin.is_none() && user.is_none() {
            let names: Vec<&str> = style::BUILTIN_STYLES
                .iter()
                .copied()
                .chain(self.user_styles.keys().map(String::as_str))
                .collect();
            let mut d =
                Diagnostic::error("unknown-style", a.span, format!("unknown style `{name}`"));
            if let Some(s) = did_you_mean(name, names) {
                d = d.with_help(format!("did you mean `{s}`?"));
            }
            self.diags.push(d);
            return None;
        }
        Some(
            builtin
                .unwrap_or_default()
                .overlay(&user.unwrap_or_default()),
        )
    }

    fn text_props(&mut self, t: &ast::Text) -> TextProps {
        let a = Attrs::new(&t.attrs, "text", TEXT_KEYS, &mut self.diags);
        let size = a
            .get("size")
            .and_then(|x| textlayout::text_size(x, &mut self.diags))
            .unwrap_or(14.0);
        let muted = a
            .get("muted")
            .is_some_and(|x| attrs::flag(x, &mut self.diags));
        let color = a
            .get("color")
            .and_then(|x| textlayout::text_color(x, &mut self.diags))
            .unwrap_or_else(|| {
                let d = Style::default();
                if muted {
                    d.muted
                } else {
                    d.color
                }
            });
        let align = match a
            .get("align")
            .and_then(|x| attrs::word(x, &["left", "center", "right"], &mut self.diags))
        {
            Some("left") => Align::Left,
            Some("right") => Align::Right,
            _ => Align::Center,
        };
        let content = t.content.clone();
        TextProps {
            content,
            size,
            color,
            align,
        }
    }

    fn line_props(&mut self, l: &ast::Line) -> LineProps {
        let a = Attrs::new(&l.attrs, "line", LINE_KEYS, &mut self.diags);
        let marker = |m: Option<&str>| match m {
            Some("arrow") => Marker::Arrow,
            Some("open") => Marker::Open,
            Some("diamond") => Marker::Diamond,
            Some("dot") => Marker::Dot,
            _ => Marker::None,
        };
        let start = marker(
            a.get("start")
                .and_then(|x| attrs::word(x, MARKERS, &mut self.diags)),
        );
        let end = marker(
            a.get("end")
                .and_then(|x| attrs::word(x, MARKERS, &mut self.diags)),
        );
        let mut props = StyleProps {
            font_size: Some(12.0),
            ..StyleProps::default()
        };
        if let Some(x) = a.get("style") {
            if let Some(named) = self.named_style(x) {
                props = props.overlay(&named.line_keys());
            }
        }
        props = props.overlay(&attrs::style_props(&a, &mut self.diags));
        let mut labels: Vec<Label> = Vec::new();
        for x in a.all("label") {
            let t = x.at_t.unwrap_or(0.5);
            if !(0.0..=1.0).contains(&t) {
                self.diags.push(Diagnostic::error(
                    "attr",
                    x.span,
                    format!("label position {t} is outside 0..1"),
                ));
                continue;
            }
            if labels.iter().any(|l| l.t == t) {
                self.diags.push(Diagnostic::error(
                    "attr",
                    x.span,
                    format!("duplicate label at t={t}"),
                ));
                continue;
            }
            if let Some(text) = attrs::text(x, &mut self.diags) {
                labels.push(Label {
                    t,
                    text,
                    at: Pt::default(),
                    rect: Rect::default(),
                    lines: Vec::new(),
                    span: x.span,
                });
            }
        }
        let ortho = a
            .get("ortho")
            .is_some_and(|x| attrs::flag(x, &mut self.diags));
        let allow = a
            .get("allow")
            .map(|x| self.allow_list(x, true))
            .unwrap_or_default();
        let reason = a
            .get("reason")
            .and_then(|x| attrs::text(x, &mut self.diags))
            .map(|r| markup::unescape(&r));
        let radius = props.radius.unwrap_or(0.0);
        LineProps {
            start,
            end,
            style: props.resolve(),
            labels,
            ortho,
            allow,
            reason,
            radius,
        }
    }

    fn build(&mut self, scene: &mut Scene) {
        for idx in 0..self.entries.len() {
            let entry = &self.entries[idx];
            if self.skipped.contains(&idx)
                || entry.parent.is_some_and(|p| self.skipped.contains(&p))
            {
                self.skipped.insert(idx);
                continue;
            }

            let (item, depth) = (entry.item, entry.depth);
            let parent = entry
                .parent
                .and_then(|p| self.entries[p].item.id())
                .map(|i| i.name.clone());
            match item {
                Item::Box(n) => {
                    self.record_edges(idx, &n.at);
                    self.push_shape(scene, idx, n, ShapeKind::Box, parent, depth)
                }
                Item::Group(g) => {
                    self.record_edges(idx, &g.node.at);
                    self.push_shape(scene, idx, &g.node, ShapeKind::Group, parent, depth)
                }
                Item::Image(i) => {
                    self.record_edges(idx, &i.at);
                    self.push_image(scene, idx, i)
                }
                Item::Junction(j) => {
                    self.record_edges(idx, &j.at);
                    if let Some(r) = self.place(idx) {
                        let dot = self.dots.get(&idx).copied().unwrap_or(false);
                        let origin = self.frame_origin(idx).unwrap_or_default();
                        scene.junctions.push(Junction {
                            id: j.id.name.clone(),
                            at: r.origin(),
                            dot,
                            parent,
                            origin,
                            span: j.span,
                        });
                        self.record_point(&j.id.name, &j.at);
                    }
                }
                Item::Text(t) => {
                    let Some(origin) = self.frame_origin(idx) else {
                        continue;
                    };
                    let Some(at) = self.eval_point(&t.at, origin) else {
                        continue;
                    };
                    let p = &self.texts[&idx];
                    let Some(block) =
                        textlayout::measure_at(&t.content, p.size, false, t.span, &mut self.diags)
                    else {
                        continue;
                    };
                    let (lines, rect) = block.place_at(at.x, at.y, textlayout::halign(p.align));
                    scene.texts.push(Text {
                        id: t.id.as_ref().map(|i| i.name.clone()),
                        content: p.content.clone(),
                        at,
                        size: p.size,
                        color: p.color.clone(),
                        align: p.align,
                        rect,
                        lines,
                        parent,
                        origin,
                        span: t.span,
                    });
                    let owner =
                        t.id.as_ref()
                            .map_or_else(|| format!("text@L{}", t.span.line), |i| i.name.clone());
                    self.record_point(&owner, &t.at);
                }
                Item::Line(l) => {
                    if let Some(line) = self.build_line(idx, l, parent) {
                        scene.lines.push(line);
                    }
                }
            }
        }
        scene.placement = std::mem::take(&mut self.edges);
        scene.literals = std::mem::take(&mut self.literals);
    }

    fn push_shape(
        &mut self,
        scene: &mut Scene,
        idx: usize,
        n: &'a ast::Node,
        kind: ShapeKind,
        parent: Option<String>,
        depth: usize,
    ) {
        let Some(rect) = self.place(idx) else { return };
        let origin = self.frame_origin(idx).unwrap_or_default();
        let block_at: Vec<Option<Pt>> = n
            .block
            .iter()
            .flatten()
            .map(|item| {
                item.at
                    .as_ref()
                    .and_then(|at| self.eval_point(at, rect.origin()))
            })
            .collect();
        let p = &self.nodes[&idx];
        let shape = shape_name(p);
        let form = self.form(&shape, rect);
        let svg = self.svg_shape(&shape);
        let icon = p.icon;
        let icon_inset = icon.map_or((0.0, 0.0), |ic| {
            let d = ic.size + p.style.padding;
            if kind == ShapeKind::Group || ic.pos == IconPos::Left {
                (d, 0.0)
            } else {
                (0.0, d)
            }
        });
        let text = textlayout::layout_shape(
            &ShapeInput {
                id: &n.id.name,
                node: n,
                kind,
                shape: &shape,
                custom_text_area: self.lib.shapes.get(&shape).and_then(LibShape::text_area),
                text_below: svg.is_some_and(|s| s.below),
                icon_inset,
                rect,
                style: &p.style,
                props: &p.text,
                block_at: &block_at,
            },
            &mut self.diags,
        );
        scene.shapes.push(Shape {
            id: n.id.name.clone(),
            kind,
            outline: p.outline,
            form,
            shape,
            image: svg.map(|s| ImageRef {
                asset: s.asset,
                rect,
                fit: Fit::Stretch,
            }),
            icon: icon.map(|ic| {
                let pad = p.style.padding;
                let s = ic.size;
                let lh = p.style.font_size * 1.25;
                let r = match (kind, ic.pos) {
                    (ShapeKind::Group, _) => {
                        Rect::new(rect.x + pad, rect.y + pad + (lh - s) / 2.0, s, s)
                    }
                    (_, IconPos::Left) => Rect::new(rect.x + pad, rect.center().y - s / 2.0, s, s),
                    (_, IconPos::Top) => Rect::new(rect.center().x - s / 2.0, rect.y + pad, s, s),
                };
                ImageRef {
                    asset: ic.asset,
                    rect: r,
                    fit: Fit::Contain,
                }
            }),
            label: n.label.clone(),
            rect,
            parent,
            depth,
            style: p.style.clone(),
            text_area: text.area,
            text_below: svg.is_some_and(|s| s.below),
            text: text.items,
            title_band: text.title_band,
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: n.span,
        });
        self.record_node(n);
    }

    fn push_image(&mut self, scene: &mut Scene, idx: usize, i: &ast::Image) {
        let Some(rect) = self.place(idx) else { return };
        let origin = self.frame_origin(idx).unwrap_or_default();
        let p = &self.nodes[&idx];
        let props = p.image.expect("image props");
        scene.shapes.push(Shape {
            id: i.id.name.clone(),
            kind: ShapeKind::Image,
            outline: Outline::Rect,
            form: None,
            shape: "image".into(),
            image: props.asset.map(|asset| ImageRef {
                asset,
                rect,
                fit: props.fit,
            }),
            icon: None,
            label: None,
            rect,
            parent: None,
            depth: 0,
            style: p.style.clone(),
            text_area: rect,
            text_below: false,
            text: Vec::new(),
            title_band: 0.0,
            allow: p.allow.clone(),
            reason: p.reason.clone(),
            origin,
            span: i.span,
        });
        self.record_point(&i.id.name, &i.at);
        self.literal(&i.id.name, "width", i.size.0, i.span);
        self.literal(&i.id.name, "height", i.size.1, i.span);
    }

    fn name(&self, idx: usize) -> &str {
        self.entries[idx]
            .item
            .id()
            .map_or("?", |id| id.name.as_str())
    }

    /// The placeable entries `idx`'s position depends on: its group, then each id in its `at`.
    fn placement_deps(&self, idx: usize) -> Vec<usize> {
        let at = match self.entries[idx].item {
            Item::Box(n) => &n.at,
            Item::Group(g) => &g.node.at,
            Item::Image(i) => &i.at,
            Item::Junction(j) => &j.at,
            Item::Text(_) | Item::Line(_) => return Vec::new(),
        };
        let refs: Vec<&ast::Ref> = match at {
            Point::Ref { r, .. } => vec![r],
            Point::Lit { x, y, .. } => [x, y]
                .into_iter()
                .filter_map(|c| match c {
                    Coord::Axis { r, .. } => Some(r),
                    Coord::Num(_) => None,
                })
                .collect(),
        };
        let placeable = |i: &usize| !matches!(self.entries[*i].item, Item::Text(_) | Item::Line(_));
        self.entries[idx]
            .parent
            .into_iter()
            .chain(
                refs.into_iter()
                    .filter_map(|r| self.ids.get(&r.id.name).copied()),
            )
            .filter(placeable)
            .collect()
    }

    /// Reports placement cycles and over-long reference chains before anything is placed,
    /// whatever the statement order, and marks the entries involved as unplaceable.
    fn placement_graph(&mut self) {
        let n = self.entries.len();
        let deps: Vec<Vec<usize>> = (0..n)
            .map(|i| {
                if self.skipped.contains(&i) {
                    Vec::new()
                } else {
                    self.placement_deps(i)
                }
            })
            .collect();
        let sccs = tarjan(&deps);
        let mut cyclic = vec![false; n];
        for scc in &sccs {
            let looped = scc.len() > 1 || deps[scc[0]].contains(&scc[0]);
            if !looped {
                continue;
            }
            for &i in scc {
                cyclic[i] = true;
                self.placed.insert(i, None);
            }
            let start = *scc.iter().min().expect("non-empty");
            let cycle = shortest_cycle(start, &deps, scc);
            self.report_cycle(&cycle);
        }

        // `depth[i]`: references in the longest chain from `i`; `next[i]`: its next link.
        let mut depth = vec![0usize; n];
        let mut next: Vec<Option<usize>> = vec![None; n];
        for scc in &sccs {
            let i = scc[0];
            if cyclic[i] {
                continue;
            }
            for &d in &deps[i] {
                if !cyclic[d] && depth[d] + 1 > depth[i] {
                    depth[i] = depth[d] + 1;
                    next[i] = Some(d);
                }
            }
        }
        let Some(head) = (0..n)
            .filter(|&i| depth[i] > MAX_PLACEMENT_DEPTH)
            .max_by(|&a, &b| depth[a].cmp(&depth[b]).then(b.cmp(&a)))
        else {
            return;
        };
        let mut tail = head;
        while let Some(t) = next[tail] {
            tail = t;
        }
        self.diags.push(
            Diagnostic::error(
                "placement-cycle",
                self.entries[head].item.at_span(),
                format!(
                    "placement chain from `{}` to `{}` is {} references deep; the limit is {MAX_PLACEMENT_DEPTH}",
                    self.name(head),
                    self.name(tail),
                    depth[head]
                ),
            )
            .with_help("place some shapes in this chain with literal coordinates"),
        );
        for (i, &d) in depth.iter().enumerate() {
            if d > MAX_PLACEMENT_DEPTH {
                self.placed.insert(i, None);
            }
        }
    }

    fn report_cycle(&mut self, cycle: &[usize]) {
        let names: Vec<&str> = cycle.iter().map(|&i| self.name(i)).collect();
        let first = names[0];
        let message = if names.len() <= CYCLE_SHOWN {
            format!("placement cycle: {} -> {first}", names.join(" -> "))
        } else {
            format!(
                "placement cycle of {} shapes: {} -> … -> {} -> {first}",
                names.len(),
                names[..4].join(" -> "),
                names[names.len() - 1]
            )
        };
        self.diags.push(
            Diagnostic::error(
                "placement-cycle",
                self.entries[cycle[0]].item.at_span(),
                message,
            )
            .with_help("place one of these with literal coordinates"),
        );
    }

    /// Places an entry (box, group or junction), resolving dependencies depth-first.
    fn place(&mut self, idx: usize) -> Option<Rect> {
        if let Some(r) = self.placed.get(&idx) {
            return *r;
        }
        if self.on_stack.len() > MAX_PLACEMENT_DEPTH || !self.on_stack.insert(idx) {
            return None;
        }
        let computed = self.compute_place(idx);
        self.on_stack.remove(&idx);
        let result = if self.placed.get(&idx) == Some(&None) {
            None
        } else {
            computed
        };
        self.placed.insert(idx, result);
        result
    }

    fn pin(&mut self, a: &ast::Attr) -> Option<(&'static str, Option<f64>)> {
        let ast::Value::AnchorAt(name, t) = &a.value else {
            return attrs::word(a, anchor::ANCHORS, &mut self.diags).map(|n| (n, None));
        };
        let Some(&name) = anchor::ANCHORS.iter().find(|n| **n == name.as_str()) else {
            let mut d =
                Diagnostic::error("attr", a.span, format!("unknown anchor `{name}` in `pin`"));
            if let Some(s) = did_you_mean(name, anchor::ANCHORS.iter().copied()) {
                d = d.with_help(format!("did you mean `{s}`?"));
            }
            self.diags.push(d);
            return None;
        };
        let unit = Rect::new(0.0, 0.0, 1.0, 1.0);
        if let Err((msg, help)) = anchor::resolve(unit, 0.0, "pin", name, Some(*t)) {
            let mut d = Diagnostic::error("unknown-anchor", a.span, format!("{msg} in `pin`"));
            if let Some(h) = help {
                d = d.with_help(h);
            }
            self.diags.push(d);
            return None;
        }
        Some((name, Some(*t)))
    }

    fn compute_place(&mut self, idx: usize) -> Option<Rect> {
        let origin = self.frame_origin(idx)?;
        match self.entries[idx].item {
            Item::Box(n) => self.place_node(idx, &n.at, n.size, origin),
            Item::Group(g) => self.place_node(idx, &g.node.at, g.node.size, origin),
            Item::Image(i) => self.place_node(idx, &i.at, i.size, origin),
            Item::Junction(j) => self
                .eval_point(&j.at, origin)
                .map(|p| Rect::new(p.x, p.y, 0.0, 0.0)),
            Item::Text(_) | Item::Line(_) => None,
        }
    }

    fn place_node(
        &mut self,
        idx: usize,
        at: &'a Point,
        size: (f64, f64),
        origin: Pt,
    ) -> Option<Rect> {
        if !self.nodes[&idx].size_ok {
            return None;
        }
        let at = self.eval_point(at, origin)?;
        let (w, h) = size;

        let p = &self.nodes[&idx];
        let local = Rect::new(0.0, 0.0, w, h);
        let (name, t) = p.pin;
        let radius = p.style.radius;
        let shape = shape_name(p);
        let form = self.form(&shape, local);
        let off =
            form::resolve_anchor(form.as_ref(), local, radius, "pin", name, t).unwrap_or_default();
        Some(Rect::new(at.x - off.x, at.y - off.y, w, h))
    }

    /// Canvas position of the frame an entry's literal coordinates are written in.
    fn frame_origin(&mut self, idx: usize) -> Option<Pt> {
        match self.entries[idx].parent {
            None => Some(Pt::new(0.0, 0.0)),
            Some(p) => self.place(p).map(Rect::origin),
        }
    }

    fn eval_point(&mut self, p: &'a Point, origin: Pt) -> Option<Pt> {
        match p {
            Point::Lit { x, y, span } => {
                let x = self.eval_coord(x, origin.x)?;
                let y = self.eval_coord(y, origin.y)?;
                self.finite(&[x, y], *span).then(|| Pt::new(x, y))
            }
            Point::Ref { r, offset, span } => {
                let base = self.eval_ref(r)?;
                let (dx, dy) = offset.unwrap_or((0.0, 0.0));
                let p = base + Pt::new(dx, dy);
                self.finite(&[p.x, p.y], *span).then_some(p)
            }
        }
    }

    fn eval_coord(&mut self, c: &'a Coord, origin: f64) -> Option<f64> {
        match c {
            Coord::Num(n) => Some(origin + n),
            Coord::Axis { r, axis } => {
                let p = self.eval_ref(r)?;
                Some(match axis {
                    Axis::X => p.x,
                    Axis::Y => p.y,
                })
            }
        }
    }

    fn eval_target(&mut self, t: &'a Target, origin: Pt, axis: Axis) -> Option<f64> {
        match (t, axis) {
            (Target::Num(n), Axis::X) => Some(origin.x + n),
            (Target::Num(n), Axis::Y) => Some(origin.y + n),
            (Target::Point(p), Axis::X) => self.eval_point(p, origin).map(|p| p.x),
            (Target::Point(p), Axis::Y) => self.eval_point(p, origin).map(|p| p.y),
        }
    }

    fn eval_ref(&mut self, r: &'a ast::Ref) -> Option<Pt> {
        let name = &r.id.name;
        let Some(&idx) = self.ids.get(name) else {
            if self.skipped_ids.contains(name) {
                return None;
            }
            let d = self.unknown_id(name, r.id.span);
            self.diags.push(d);
            return None;
        };
        match self.entries[idx].item {
            Item::Junction(_) => {
                if let Some(a) = &r.anchor {
                    self.diags.push(
                        Diagnostic::error(
                            "unknown-anchor",
                            a.name.span,
                            format!("`{name}` is a junction and has no anchors"),
                        )
                        .with_help(format!("use `{name}` on its own")),
                    );
                    return None;
                }
                self.place(idx).map(Rect::origin)
            }
            Item::Box(_) | Item::Group(_) | Item::Image(_) => {
                let Some(a) = &r.anchor else {
                    self.diags.push(
                        Diagnostic::error(
                            "unknown-anchor",
                            r.id.span,
                            format!("`{name}` needs an anchor"),
                        )
                        .with_help(format!(
                            "e.g. `{name}.left`, `{name}.top` or `{name}.center`"
                        )),
                    );
                    return None;
                };
                let rect = self.place(idx)?;
                let p = &self.nodes[&idx];
                let form = p.shape.as_deref().and_then(|s| self.form(s, rect));
                match form::resolve_anchor(
                    form.as_ref(),
                    rect,
                    p.style.radius,
                    name,
                    &a.name.name,
                    a.t,
                ) {
                    Ok(p) => Some(p),
                    Err((msg, help)) => {
                        let mut d = Diagnostic::error("unknown-anchor", a.name.span, msg);
                        if let Some(h) = help {
                            d = d.with_help(h);
                        }
                        self.diags.push(d);
                        None
                    }
                }
            }
            Item::Text(_) | Item::Line(_) => {
                self.diags.push(Diagnostic::error(
                    "unknown-id",
                    r.id.span,
                    format!("`{name}` is a text or line; only boxes, groups and junctions can be referenced"),
                ));
                None
            }
        }
    }

    fn build_line(&mut self, idx: usize, l: &'a ast::Line, parent: Option<String>) -> Option<Line> {
        let origin = self.frame_origin(idx)?;
        let mut cur = self.eval_point(&l.start, origin)?;
        let mut raw: Vec<(PathSeg, usize)> = Vec::new();
        let mut segments: Vec<Segment> = Vec::new();
        let mut failed = false;
        for seg in &l.segs {
            let from = cur;
            let Some(segs) = self.expand(seg, cur, origin) else {
                failed = true;
                break;
            };
            let mut pieces = Vec::new();
            for s in segs {
                cur = s.to();
                if matches!(s, PathSeg::Line { .. }) && s.from().dist(s.to()) <= 1e-9 {
                    continue;
                }
                raw.push((s, segments.len()));
                pieces.push(s);
            }
            segments.push(Segment {
                kind: record::segment_kind(&seg.kind),
                span: seg.span,
                from,
                to: cur,
                pieces,
            });
        }
        if failed {
            return None;
        }
        if raw.is_empty() {
            self.diags.push(Diagnostic::error(
                "zero-length",
                l.span,
                "line has zero length",
            ));
            return None;
        }
        let p = self.lines.remove(&idx)?;
        if p.ortho {
            for (s, i) in &raw {
                if !s.is_axis_aligned() {
                    self.diags.push(
                        Diagnostic::error(
                            "not-ortho",
                            segments[*i].span,
                            "segment is not horizontal or vertical on a line marked `ortho`",
                        )
                        .with_help("use `hv`/`vh` elbows or `x`/`y` targets"),
                    );
                }
            }
        }
        let rounded = if p.radius > 0.0 {
            round_corners_tagged(&raw, p.radius)
        } else {
            raw
        };
        let (path, path_seg): (Vec<PathSeg>, Vec<usize>) = rounded.into_iter().unzip();
        let mut labels = p.labels;
        let bold = p.style.font_weight == crate::style::FontWeight::Bold;
        labels.retain_mut(|lab| {
            let Some(block) = textlayout::measure_at(
                &lab.text,
                p.style.font_size,
                bold,
                lab.span,
                &mut self.diags,
            ) else {
                return false;
            };
            lab.at = point_at(&path, lab.t);
            (lab.lines, lab.rect) =
                block.place_at(lab.at.x, lab.at.y, kashika_text::HAlign::Center);
            true
        });
        let (id, named) = match &l.id {
            Some(i) => (i.name.clone(), true),
            None => (format!("line@L{}", l.span.line), false),
        };
        self.record_line(&id, l);
        Some(Line {
            id,
            named,
            path,
            segments,
            path_seg,
            start_literal: record::is_literal(&l.start),
            end_literal: record::end_is_literal(&l.segs),
            start: p.start,
            end: p.end,
            style: p.style,
            labels,
            ortho: p.ortho,
            allow: p.allow,
            reason: p.reason,
            parent,
            origin,
            span: l.span,
        })
    }

    fn expand(&mut self, seg: &'a ast::Seg, cur: Pt, origin: Pt) -> Option<Vec<PathSeg>> {
        let line = |a: Pt, b: Pt| PathSeg::Line { from: a, to: b };
        Some(match &seg.kind {
            SegKind::To(p) => vec![line(cur, self.eval_point(p, origin)?)],
            SegKind::H(dx) => {
                let x = cur.x + dx;
                self.finite(&[x], seg.span).then_some(())?;
                vec![line(cur, Pt::new(x, cur.y))]
            }
            SegKind::V(dy) => {
                let y = cur.y + dy;
                self.finite(&[y], seg.span).then_some(())?;
                vec![line(cur, Pt::new(cur.x, y))]
            }
            SegKind::X(t) => {
                let x = self.eval_target(t, origin, Axis::X)?;
                self.finite(&[x], seg.span).then_some(())?;
                vec![line(cur, Pt::new(x, cur.y))]
            }
            SegKind::Y(t) => {
                let y = self.eval_target(t, origin, Axis::Y)?;
                self.finite(&[y], seg.span).then_some(())?;
                vec![line(cur, Pt::new(cur.x, y))]
            }
            SegKind::Hv(p) => {
                let p = self.eval_point(p, origin)?;
                let corner = Pt::new(p.x, cur.y);
                vec![line(cur, corner), line(corner, p)]
            }
            SegKind::Vh(p) => {
                let p = self.eval_point(p, origin)?;
                let corner = Pt::new(cur.x, p.y);
                vec![line(cur, corner), line(corner, p)]
            }
            SegKind::Curve { side, amount, to } => {
                let to = self.eval_point(to, origin)?;
                self.finite(&[*amount], seg.span).then_some(())?;
                if *amount < 0.0 {
                    self.diags.push(Diagnostic::error(
                        "degenerate-curve",
                        seg.span,
                        "curve amount must not be negative; use `left`/`right` to choose the side",
                    ));
                    return None;
                }
                if cur.dist(to) <= TOLERANCE {
                    self.diags.push(
                        Diagnostic::error(
                            "degenerate-curve",
                            seg.span,
                            "curve starts and ends at the same point",
                        )
                        .with_help("use `curve via (c1) (c2) <point>` for loops"),
                    );
                    return None;
                }
                let d = (to - cur).norm();
                // Screen-left of the travel direction in y-down coordinates.
                let normal = match side {
                    Side::Left => Pt::new(d.y, -d.x),
                    Side::Right => Pt::new(-d.y, d.x),
                };
                let ctrl = cur.lerp(to, 0.5) + normal * (2.0 * amount);
                vec![PathSeg::Quad {
                    from: cur,
                    ctrl,
                    to,
                }]
            }
            SegKind::CurveVia { c1, c2, to } => {
                let c1 = self.eval_point(c1, origin)?;
                let c2 = self.eval_point(c2, origin)?;
                let to = self.eval_point(to, origin)?;
                if [c1, c2, to].iter().all(|p| p.dist(cur) <= TOLERANCE) {
                    self.diags.push(Diagnostic::error(
                        "degenerate-curve",
                        seg.span,
                        "curve has zero size",
                    ));
                    return None;
                }
                vec![PathSeg::Cubic {
                    from: cur,
                    c1,
                    c2,
                    to,
                }]
            }
        })
    }
}

fn shape_name(p: &NodeProps) -> String {
    p.shape.clone().unwrap_or_else(|| {
        if p.outline == Outline::Rounded {
            "rounded".into()
        } else {
            "rect".into()
        }
    })
}

/// Strongly connected components of `deps`, each listed after every component it depends on.
fn tarjan(deps: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = deps.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on = vec![false; n];
    let mut stack = Vec::new();
    let mut sccs = Vec::new();
    let mut counter = 0;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut k)) = work.last_mut() {
            if *k == 0 && index[v] == usize::MAX {
                index[v] = counter;
                low[v] = counter;
                counter += 1;
                stack.push(v);
                on[v] = true;
            }
            if let Some(&w) = deps[v].get(*k) {
                *k += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            work.pop();
            if let Some(&(u, _)) = work.last() {
                low[u] = low[u].min(low[v]);
            }
            if low[v] == index[v] {
                let mut scc = Vec::new();
                while let Some(w) = stack.pop() {
                    on[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                scc.sort_unstable();
                sccs.push(scc);
            }
        }
    }
    sccs
}

/// The shortest cycle through `start` inside its component `scc`, starting at `start`.
fn shortest_cycle(start: usize, deps: &[Vec<usize>], scc: &[usize]) -> Vec<usize> {
    let mut prev: HashMap<usize, usize> = HashMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(v) = queue.pop_front() {
        for &w in &deps[v] {
            if w == start {
                let mut cycle = vec![v];
                let mut cur = v;
                while cur != start {
                    cur = prev[&cur];
                    cycle.push(cur);
                }
                cycle.reverse();
                return cycle;
            }
            if scc.binary_search(&w).is_ok() && !prev.contains_key(&w) {
                prev.insert(w, v);
                queue.push_back(w);
            }
        }
    }
    vec![start]
}
