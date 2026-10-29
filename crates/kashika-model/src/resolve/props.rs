use kashika_diag::{Diagnostic, Span, did_you_mean};
use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self};

use crate::attrs::{self, Attrs, Min};
use crate::form::Form;
use crate::library::{self, LibShape, SvgShape};
use crate::markup;
use crate::scene::*;
use crate::shapes;
use crate::style::{self, STYLE_KEYS, Style, StyleProps};
use crate::textlayout::{self, SHAPE_TEXT_KEYS, ShapeInput, ShapeTextProps};

use super::{
    IMAGE_KEYS, IconPos, IconProps, ImageProps, Item, LINE_KEYS, LineProps, NODE_KEYS, NodeProps,
    Resolver, TEXT_KEYS, TextProps,
};

impl NodeProps {
    /// How far the icon pushes the text area from the left and from the top edge.
    pub(super) fn icon_inset(&self, group: bool) -> (f64, f64) {
        self.icon.map_or((0.0, 0.0), |ic| {
            let pad = self.style.padding;
            if group || ic.pos == IconPos::Left {
                (ic.size + pad.x, 0.0)
            } else {
                (0.0, ic.size + pad.y)
            }
        })
    }

    /// The shape drawn: the `shape=` name, else `rounded` or `rect` from the outline.
    pub(super) fn drawn_shape(&self) -> String {
        self.shape.clone().unwrap_or_else(|| {
            if self.outline == Outline::Rounded {
                "rounded".into()
            } else {
                "rect".into()
            }
        })
    }
}

impl<'a> Resolver<'a> {
    /// Reads every entry's attributes into its props, loading the files they name.
    /// Skipped entries (duplicate ids and their children) get none.
    pub(super) fn validate(&mut self) {
        for idx in 0..self.entries.len() {
            if self.skipped.contains(&idx) {
                continue;
            }
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

    /// `allow=` and `reason=`, shared by boxes, groups, images and lines.
    fn allow_reason(&mut self, a: &Attrs<'_>) -> (Vec<Allow>, Option<String>) {
        let allow = a
            .get("allow")
            .map(|x| self.allow_list(x, true))
            .unwrap_or_default();
        let reason = a
            .get("reason")
            .and_then(|x| attrs::text(x, &mut self.diags))
            .map(|r| markup::unescape(&r));
        (allow, reason)
    }

    pub(super) fn node_props(&mut self, n: &ast::Node, group: bool) -> NodeProps {
        let what = if group { "group" } else { "box" };
        let keys: Vec<&str> = NODE_KEYS
            .iter()
            .filter(|k| !group || !matches!(**k, "shape" | "layer"))
            .chain(STYLE_KEYS)
            .chain(SHAPE_TEXT_KEYS)
            .copied()
            .collect();
        let a = Attrs::new(&n.attrs, what, &keys, &mut self.diags);

        let shape = a.get("shape").and_then(|x| self.shape_attr(x));
        let mut props = StyleProps::default();
        let mut color = None;
        if group {
            let user = self
                .user_styles
                .get("boundary")
                .cloned()
                .unwrap_or_default();
            color = user.color.clone();
            props = props
                .overlay(&style::builtin("boundary").unwrap_or_default())
                .overlay(&user);
        }
        if let Some(x) = a.get("style") {
            if let Some(named) = self.named_style(x) {
                if group && named.shape.is_some() {
                    self.diags.push(
                        Diagnostic::error(
                            "attr",
                            x.span,
                            "this style sets `shape`, but groups take no shape",
                        )
                        .with_help("use a style without `shape`, or drop `shape` from it"),
                    );
                }
                color = named.color.clone().or(color);
                props = props.overlay(&named);
            }
        }
        let own = attrs::style_props(&a, &mut self.diags);
        color = own.color.clone().or(color);
        props = props.overlay(&own);
        let shape = if group {
            None
        } else {
            shape.or_else(|| props.shape.clone())
        };
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
        let (allow, reason) = self.allow_reason(&a);
        let style = props.resolve();
        let icon_color = color
            .as_deref()
            .filter(|c| *c != "none")
            .unwrap_or(style::ICON_COLOR);
        let icon = self.icon_props(&a, group, style.font_size, icon_color);
        let layer = self.layer(&a);
        let mut p = NodeProps {
            outline,
            shape,
            style,
            pin,
            size: None,
            text,
            allow,
            reason,
            icon,
            image: None,
            layer,
        };
        p.size = self.node_size(n, group, &p, props.size);
        p
    }

    /// The size of a box or group: its `size`, else its style's, with each `auto` axis (or
    /// both, when neither gives one) fitted to its text and rounded up to the `grid`.
    fn node_size(
        &mut self,
        n: &ast::Node,
        group: bool,
        p: &NodeProps,
        styled: Option<ast::Size>,
    ) -> Option<(f64, f64)> {
        let what = format!("`{}`", n.id.name);
        let ast::Size { w, h } = n.size.or(styled).unwrap_or(ast::Size { w: None, h: None });
        if let (Some(w), Some(h)) = (w, h) {
            return self.size_ok((w, h), n.span, &what).then_some((w, h));
        }
        if group {
            self.diags.push(
                Diagnostic::error("size", n.span, format!("group {what} needs a size"))
                    .with_help("groups don't fit their contents; give it a size: `size (w,h)`"),
            );
            return None;
        }
        if !self.size_ok((w.unwrap_or(1.0), h.unwrap_or(1.0)), n.span, &what) {
            return None;
        }
        let shape = p.drawn_shape();
        let input = ShapeInput {
            id: &n.id.name,
            node: n,
            kind: ShapeKind::Box,
            shape: &shape,
            custom_text_area: self.lib.shapes.get(&shape).and_then(LibShape::text_area),
            text_below: self.svg_shape(&shape).is_some_and(|s| s.below),
            icon_inset: p.icon_inset(false),
            rect: Rect::new(0.0, 0.0, w.unwrap_or(0.0), h.unwrap_or(0.0)),
            style: &p.style,
            props: &p.text,
            block_at: &[],
        };
        let Some(fit) = textlayout::fit_size(&input) else {
            let why = if input.text_below {
                "its text sits under it (`text-area=below`)"
            } else {
                "it has no text"
            };
            self.diags.push(
                Diagnostic::error("size", n.span, format!("{what} has no size to fit: {why}"))
                    .with_help("give it a size: `size (w,h)`"),
            );
            return None;
        };
        let grid = self.grid;
        let snap = |v: f64| grid.map_or(v, |g| (v / g).ceil() * g);
        let size = (
            w.unwrap_or_else(|| snap(fit.0)),
            h.unwrap_or_else(|| snap(fit.1)),
        );
        self.size_ok(size, n.span, &what).then_some(size)
    }

    fn layer(&mut self, a: &Attrs<'_>) -> Layer {
        match a
            .get("layer")
            .and_then(|x| attrs::word(x, &["under", "over"], &mut self.diags))
        {
            Some("under") => Layer::Under,
            Some("over") => Layer::Over,
            _ => Layer::Normal,
        }
    }

    pub(super) fn icon_props(
        &mut self,
        a: &Attrs<'_>,
        group: bool,
        font_size: f64,
        color: &str,
    ) -> Option<IconProps> {
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
        let size = a
            .get("icon-size")
            .and_then(|x| attrs::num_in(x, Min::Positive, &mut self.diags))
            .unwrap_or(if group { font_size * 1.25 } else { 24.0 });
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
        let root = self.loader.root().to_path_buf();
        let asset = match self.store.load_icon(&self.loader, &root, &name, color) {
            Ok(i) => i,
            Err(e) => {
                self.file_error(x.span, e);
                return None;
            }
        };
        Some(IconProps { asset, pos, size })
    }

    pub(super) fn image_props(&mut self, i: &ast::Image) -> NodeProps {
        let a = Attrs::new(&i.attrs, "image", IMAGE_KEYS, &mut self.diags);
        let size = self
            .size_ok(i.size, i.span, &format!("`{}`", i.id.name))
            .then_some(i.size);
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
        let (allow, reason) = self.allow_reason(&a);
        let layer = self.layer(&a);
        let asset = self.load_asset(&i.file, i.file_span);
        NodeProps {
            outline: Outline::Rect,
            shape: None,
            style: Style::default(),
            pin,
            size,
            text: ShapeTextProps::default(),
            allow,
            reason,
            icon: None,
            image: Some(ImageProps { asset, fit }),
            layer,
        }
    }

    pub(super) fn load_asset(&mut self, name: &str, span: Span) -> Option<usize> {
        let root = self.loader.root().to_path_buf();
        match self.store.load(&self.loader, &root, name) {
            Ok(i) => Some(i),
            Err(e) => {
                self.file_error(span, e);
                None
            }
        }
    }

    /// The shape a `shape=` attribute names, if it exists.
    pub(super) fn shape_attr(&mut self, a: &ast::Attr) -> Option<String> {
        library::shape_name(a, "", &self.lib.shapes, &mut self.diags)
    }

    /// The outline and drawing of a shape name on `rect`: a custom definition, else a
    /// built-in sized for `rect`. `None` for `rect` and `rounded`.
    pub(super) fn form(&self, name: &str, rect: Rect) -> Option<Form> {
        match self.lib.shapes.get(name) {
            Some(LibShape::Unit(u)) => Some(Form::from_unit(u, rect)),
            Some(LibShape::Svg(s)) => Some(Form::from_svg(self.store.get(s.asset), s.hull, rect)),
            None => shapes::builtin(name, rect.w, rect.h).map(|u| Form::from_unit(&u, rect)),
        }
    }

    pub(super) fn svg_shape(&self, name: &str) -> Option<SvgShape> {
        match self.lib.shapes.get(name) {
            Some(LibShape::Svg(s)) => Some(*s),
            _ => None,
        }
    }

    pub(super) fn named_style(&mut self, a: &ast::Attr) -> Option<StyleProps> {
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

    pub(super) fn text_props(&mut self, t: &ast::Text) -> TextProps {
        let a = Attrs::new(&t.attrs, "text", TEXT_KEYS, &mut self.diags);
        let size = attrs::aliased(&a, "font-size", "size", &mut self.diags)
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
                if muted { d.muted } else { d.color }
            });
        let align = match attrs::aliased(&a, "text-align", "align", &mut self.diags)
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

    pub(super) fn line_props(&mut self, l: &ast::Line) -> LineProps {
        let a = Attrs::new(&l.attrs, "line", LINE_KEYS, &mut self.diags);
        let mut marker = |key: &str| {
            a.get(key)
                .and_then(|x| attrs::word(x, Marker::NAMES, &mut self.diags))
                .and_then(Marker::from_name)
                .unwrap_or(Marker::None)
        };
        let start = marker("start");
        let end = match a.get("end") {
            Some(_) => marker("end"),
            None if l.arrow => Marker::Arrow,
            None => Marker::None,
        };
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
                    side: LabelSide::On,
                    span: x.span,
                });
            }
        }
        let side_attr = a.get("label-side");
        let label_side = side_attr
            .and_then(|x| attrs::word(x, LabelSide::NAMES, &mut self.diags))
            .and_then(LabelSide::from_name)
            .unwrap_or_default();
        let ortho = a
            .get("ortho")
            .is_some_and(|x| attrs::flag(x, &mut self.diags));
        let (allow, reason) = self.allow_reason(&a);
        let radius = props.radius.unwrap_or(0.0);
        LineProps {
            start,
            end,
            style: props.resolve(),
            labels,
            label_side,
            label_side_span: side_attr.map_or(l.span, |x| x.span),
            ortho,
            allow,
            reason,
            radius,
        }
    }
}
