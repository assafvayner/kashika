use std::collections::{HashMap, HashSet};

use kashika_diag::{did_you_mean, Diagnostic, Span, LINT_CODES};
use kashika_geom::{round_corners, PathSeg, Pt, Rect, TOLERANCE};
use kashika_syntax::ast::{self, Axis, Coord, Point, SegKind, Side, Stmt, Target};

use crate::anchor;
use crate::attrs::{self, Attrs};
use crate::scene::*;
use crate::style::{self, Style, StyleProps, STYLE_KEYS};

const NODE_KEYS: &[&str] = &["shape", "style", "pin"];
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
const TEXT_KEYS: &[&str] = &["size", "color", "align"];
const DIAGRAM_KEYS: &[&str] = &["kashika", "grid", "clearance", "icons", "allow"];
const MARKERS: &[&str] = &["none", "arrow", "open", "diamond", "dot"];
const SHAPES: &[&str] = &["rect", "rounded"];
const UNSUPPORTED_KEYS: &[&str] = &[
    "sublabel",
    "text-align",
    "text-valign",
    "gap",
    "icon",
    "icon-pos",
    "icon-size",
];
const UNSUPPORTED_SHAPES: &[&str] = &[
    "cylinder", "diamond", "ellipse", "hexagon", "doc", "cloud", "person",
];

#[derive(Clone, Copy)]
enum Item<'a> {
    Box(&'a ast::Node),
    Group(&'a ast::Group),
    Junction(&'a ast::Junction),
    Text(&'a ast::Text),
    Line(&'a ast::Line),
}

impl<'a> Item<'a> {
    fn id(self) -> Option<&'a ast::Ident> {
        match self {
            Item::Box(n) => Some(&n.id),
            Item::Group(g) => Some(&g.node.id),
            Item::Junction(j) => Some(&j.id),
            Item::Text(t) => t.id.as_ref(),
            Item::Line(l) => l.id.as_ref(),
        }
    }

    fn span(self) -> Span {
        match self {
            Item::Box(n) => n.span,
            Item::Group(g) => g.node.span,
            Item::Junction(j) => j.span,
            Item::Text(t) => t.span,
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
    style: Style,
    pin: &'static str,
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
    size: f64,
    color: String,
    align: Align,
}

pub fn resolve(file: &ast::File) -> (Scene, Vec<Diagnostic>) {
    let mut r = Resolver {
        entries: Vec::new(),
        ids: HashMap::new(),
        user_styles: HashMap::new(),
        nodes: HashMap::new(),
        lines: HashMap::new(),
        texts: HashMap::new(),
        dots: HashMap::new(),
        placed: HashMap::new(),
        stack: Vec::new(),
        skipped: HashSet::new(),
        diags: Vec::new(),
    };
    r.styles(file);
    r.collect(&file.stmts, None, 0);
    let mut scene = r.diagram(file);
    r.validate();
    r.build(&mut scene);
    (scene, r.diags)
}

struct Resolver<'a> {
    entries: Vec<Entry<'a>>,
    ids: HashMap<String, usize>,
    user_styles: HashMap<String, StyleProps>,
    nodes: HashMap<usize, NodeProps>,
    lines: HashMap<usize, LineProps>,
    texts: HashMap<usize, TextProps>,
    dots: HashMap<usize, bool>,
    /// Canvas rect per placed entry (junctions use a zero-size rect); `None` = failed.
    placed: HashMap<usize, Option<Rect>>,
    stack: Vec<usize>,
    skipped: HashSet<usize>,
    diags: Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
    fn diagram(&mut self, file: &ast::File) -> Scene {
        let mut scene = Scene {
            title: String::new(),
            size: (0.0, 0.0),
            grid: None,
            clearance: 6.0,
            allow: Vec::new(),
            shapes: Vec::new(),
            junctions: Vec::new(),
            texts: Vec::new(),
            lines: Vec::new(),
        };
        let mut seen = false;
        for (i, stmt) in file.stmts.iter().enumerate() {
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
            if i != 0 {
                self.diags.push(Diagnostic::error(
                    "diagram",
                    d.span,
                    "`diagram` must be the first statement",
                ));
            }
            scene.title = d.title.clone();
            scene.size = d.size;
            if self.finite(&[d.size.0, d.size.1], d.span) && (d.size.0 <= 0.0 || d.size.1 <= 0.0) {
                self.diags.push(Diagnostic::error(
                    "size",
                    d.span,
                    "diagram size must be positive",
                ));
            }

            let a = Attrs::new(&d.attrs, "diagram", DIAGRAM_KEYS, &mut self.diags);
            if let Some(x) = a.get("kashika") {
                if let Some(v) = attrs::num(x, &mut self.diags) {
                    if v != 1.0 {
                        self.diags.push(Diagnostic::error(
                            "version",
                            x.span,
                            format!("this file targets kashika={v}, but this binary implements kashika=1"),
                        ));
                    }
                }
            }
            if let Some(x) = a.get("grid") {
                scene.grid = attrs::num(x, &mut self.diags);
            }
            if let Some(x) = a.get("clearance") {
                if let Some(v) = attrs::num(x, &mut self.diags) {
                    scene.clearance = v;
                }
            }
            if let Some(x) = a.get("icons") {
                self.unsupported(&x.key.name, x.key.span);
            }
            if let Some(x) = a.get("allow") {
                scene.allow = self.allow_list(x, false);
            }
        }
        if !seen {
            let span = file.stmts.first().map_or(Span::default(), Stmt::span);
            self.diags.push(
                Diagnostic::error("diagram", span, "missing `diagram` statement")
                    .with_help("start the file with `diagram \"<title>\" size (w,h)`"),
            );
        }
        scene
    }

    fn styles(&mut self, file: &ast::File) {
        for stmt in &file.stmts {
            let Stmt::Style(s) = stmt else { continue };
            let a = Attrs::new(&s.attrs, "style", STYLE_KEYS, &mut self.diags);
            let props = attrs::style_props(&a, &mut self.diags);
            if self.user_styles.insert(s.name.clone(), props).is_some() {
                self.diags.push(Diagnostic::error(
                    "duplicate-id",
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
                Stmt::Style(_) => {
                    if parent.is_some() {
                        self.diags.push(Diagnostic::error(
                            "parse",
                            stmt.span(),
                            "`style` is only allowed at the top level",
                        ));
                    }
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
            } else if let Some(id) = item.id() {
                self.register(id, idx);
            }
            if let Stmt::Group(g) = stmt {
                self.collect(&g.children, Some(idx), depth + 1);
            }
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

    fn finite(&mut self, vals: &[f64], span: Span) -> bool {
        if vals.iter().all(|v| v.is_finite()) {
            true
        } else {
            self.diags.push(Diagnostic::error(
                "degenerate-curve",
                span,
                "value is not a finite number",
            ));
            false
        }
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
            .chain(UNSUPPORTED_KEYS)
            .copied()
            .collect();
        let a = Attrs::new(&n.attrs, what, &keys, &mut self.diags);
        for x in a.iter() {
            if UNSUPPORTED_KEYS.contains(&x.key.name.as_str()) {
                self.unsupported(&x.key.name, x.key.span);
            }
        }

        if n.size.0 <= 0.0 || n.size.1 <= 0.0 {
            self.diags.push(Diagnostic::error(
                "size",
                n.span,
                format!("`{}` must have a positive size", n.id.name),
            ));
        }
        let mut shape = None;
        if let Some(x) = a.get("shape") {
            match &x.value {
                ast::Value::Word(w) if UNSUPPORTED_SHAPES.contains(&w.as_str()) => {
                    self.unsupported(w, x.span)
                }
                _ => shape = attrs::word(x, SHAPES, &mut self.diags),
            }
        }
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
        if shape == Some("rounded") && props.radius.is_none() {
            props.radius = Some(8.0);
        }
        let outline = if shape == Some("rounded") || props.radius.unwrap_or(0.0) > 0.0 {
            Outline::Rounded
        } else {
            Outline::Rect
        };
        let pin = a
            .get("pin")
            .and_then(|x| attrs::word(x, anchor::ANCHORS, &mut self.diags))
            .unwrap_or("tl");
        NodeProps {
            outline,
            style: props.resolve(),
            pin,
        }
    }

    fn unsupported(&mut self, name: &str, span: Span) {
        self.diags.push(Diagnostic::error(
            "unsupported",
            span,
            format!("`{name}` is not supported in this version yet"),
        ));
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
            .and_then(|x| {
                let v = attrs::num(x, &mut self.diags)?;
                if v <= 0.0 {
                    self.diags.push(Diagnostic::error(
                        "size",
                        x.span,
                        "text size must be positive",
                    ));
                    return None;
                }
                Some(v)
            })
            .unwrap_or(14.0);
        let color = a
            .get("color")
            .and_then(|x| attrs::color(x, &mut self.diags))
            .unwrap_or_else(|| Style::default().color);
        let align = match a
            .get("align")
            .and_then(|x| attrs::word(x, &["left", "center", "right"], &mut self.diags))
        {
            Some("left") => Align::Left,
            Some("right") => Align::Right,
            _ => Align::Center,
        };
        TextProps { size, color, align }
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
                labels.push(Label { t, text });
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
            .and_then(|x| attrs::text(x, &mut self.diags));
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
                Item::Box(n) => self.push_shape(scene, idx, n, ShapeKind::Box, parent, depth),
                Item::Group(g) => {
                    self.push_shape(scene, idx, &g.node, ShapeKind::Group, parent, depth)
                }
                Item::Junction(j) => {
                    if let Some(r) = self.place(idx) {
                        let dot = self.dots.get(&idx).copied().unwrap_or(false);
                        scene.junctions.push(Junction {
                            id: j.id.name.clone(),
                            at: r.origin(),
                            dot,
                            parent,
                            span: j.span,
                        });
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
                    scene.texts.push(Text {
                        id: t.id.as_ref().map(|i| i.name.clone()),
                        content: t.content.clone(),
                        at,
                        size: p.size,
                        color: p.color.clone(),
                        align: p.align,
                        parent,
                        span: t.span,
                    });
                }
                Item::Line(l) => {
                    if let Some(line) = self.build_line(idx, l, parent) {
                        scene.lines.push(line);
                    }
                }
            }
        }
    }

    fn push_shape(
        &mut self,
        scene: &mut Scene,
        idx: usize,
        n: &ast::Node,
        kind: ShapeKind,
        parent: Option<String>,
        depth: usize,
    ) {
        let Some(rect) = self.place(idx) else { return };
        let p = &self.nodes[&idx];
        scene.shapes.push(Shape {
            id: n.id.name.clone(),
            kind,
            outline: p.outline,
            label: n.label.clone(),
            rect,
            parent,
            depth,
            style: p.style.clone(),
            span: n.span,
        });
    }

    /// Places an entry (box, group or junction), resolving dependencies depth-first.
    fn place(&mut self, idx: usize) -> Option<Rect> {
        if let Some(r) = self.placed.get(&idx) {
            return *r;
        }
        if let Some(pos) = self.stack.iter().position(|&i| i == idx) {
            let names: Vec<String> = self.stack[pos..]
                .iter()
                .chain(std::iter::once(&idx))
                .map(|&i| {
                    self.entries[i]
                        .item
                        .id()
                        .map_or("?".to_string(), |id| id.name.clone())
                })
                .collect();
            let span = self.entries[idx].item.span();
            self.diags.push(
                Diagnostic::error(
                    "placement-cycle",
                    span,
                    format!("placement cycle: {}", names.join(" -> ")),
                )
                .with_help("place one of these with literal coordinates"),
            );
            for &i in &self.stack[pos..] {
                self.placed.insert(i, None);
            }
            return None;
        }
        self.stack.push(idx);
        let computed = self.compute_place(idx);
        self.stack.pop();
        let result = if self.placed.get(&idx) == Some(&None) {
            None
        } else {
            computed
        };
        self.placed.insert(idx, result);
        result
    }

    fn compute_place(&mut self, idx: usize) -> Option<Rect> {
        let origin = self.frame_origin(idx)?;
        match self.entries[idx].item {
            Item::Box(n) => self.place_node(idx, n, origin),
            Item::Group(g) => self.place_node(idx, &g.node, origin),
            Item::Junction(j) => self
                .eval_point(&j.at, origin)
                .map(|p| Rect::new(p.x, p.y, 0.0, 0.0)),
            Item::Text(_) | Item::Line(_) => None,
        }
    }

    fn place_node(&mut self, idx: usize, n: &'a ast::Node, origin: Pt) -> Option<Rect> {
        let at = self.eval_point(&n.at, origin)?;
        let (w, h) = n.size;
        if !self.finite(&[at.x, at.y, w, h], n.span) {
            return None;
        }

        let off = anchor::pin_offset(self.nodes[&idx].pin, w, h);
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
            Item::Box(_) | Item::Group(_) => {
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
                let radius = corner_radius(self.nodes[&idx].style.radius, rect);
                match anchor::resolve(rect, radius, name, &a.name.name, a.t) {
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
        let mut raw: Vec<(PathSeg, Span)> = Vec::new();
        let mut failed = false;
        for seg in &l.segs {
            let Some(segs) = self.expand(seg, cur, origin) else {
                failed = true;
                break;
            };
            for s in segs {
                cur = s.to();
                if matches!(s, PathSeg::Line { .. }) && s.from().dist(s.to()) <= 1e-9 {
                    continue;
                }
                raw.push((s, seg.span));
            }
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
            for (s, span) in &raw {
                if !s.is_axis_aligned() {
                    self.diags.push(
                        Diagnostic::error(
                            "not-ortho",
                            *span,
                            "segment is not horizontal or vertical on a line marked `ortho`",
                        )
                        .with_help("use `hv`/`vh` elbows or `x`/`y` targets"),
                    );
                }
            }
        }
        let segs: Vec<PathSeg> = raw.into_iter().map(|(s, _)| s).collect();
        let path = if p.radius > 0.0 {
            round_corners(&segs, p.radius)
        } else {
            segs
        };
        let (id, named) = match &l.id {
            Some(i) => (i.name.clone(), true),
            None => (format!("line@L{}", l.span.line), false),
        };
        Some(Line {
            id,
            named,
            path,
            start: p.start,
            end: p.end,
            style: p.style,
            labels: p.labels,
            ortho: p.ortho,
            allow: p.allow,
            reason: p.reason,
            parent,
            span: l.span,
        })
    }

    fn expand(&mut self, seg: &'a ast::Seg, cur: Pt, origin: Pt) -> Option<Vec<PathSeg>> {
        let line = |a: Pt, b: Pt| PathSeg::Line { from: a, to: b };
        Some(match &seg.kind {
            SegKind::To(p) => vec![line(cur, self.eval_point(p, origin)?)],
            SegKind::H(dx) => {
                let x = cur.x + dx;
                self.finite(&[*dx, x], seg.span).then_some(())?;
                vec![line(cur, Pt::new(x, cur.y))]
            }
            SegKind::V(dy) => {
                let y = cur.y + dy;
                self.finite(&[*dy, y], seg.span).then_some(())?;
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
