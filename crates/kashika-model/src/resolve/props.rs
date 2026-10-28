use kashika_diag::{Diagnostic, Span, did_you_mean};
use kashika_geom::{Pt, Rect};
use kashika_syntax::ast::{self};

use crate::attrs::{self, Attrs};
use crate::form::Form;
use crate::library::{LibShape, SvgShape};
use crate::markup;
use crate::scene::*;
use crate::shapes::{self, BUILTIN_SHAPES};
use crate::style::{self, STYLE_KEYS, Style, StyleProps};
use crate::textlayout::{self, SHAPE_TEXT_KEYS};

use super::{
    IMAGE_KEYS, IconPos, IconProps, ImageProps, LINE_KEYS, LineProps, MARKERS, NODE_KEYS,
    NodeProps, Resolver, TEXT_KEYS, TextProps,
};

impl<'a> Resolver<'a> {
    pub(super) fn node_props(&mut self, n: &ast::Node, group: bool) -> NodeProps {
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

    pub(super) fn icon_props(
        &mut self,
        a: &Attrs<'_>,
        group: bool,
        font_size: f64,
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

    pub(super) fn image_props(&mut self, i: &ast::Image) -> NodeProps {
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

    pub(super) fn shape_name(&mut self, a: &ast::Attr) -> Option<String> {
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
                if muted { d.muted } else { d.color }
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

    pub(super) fn line_props(&mut self, l: &ast::Line) -> LineProps {
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
}

pub(super) fn shape_name(p: &NodeProps) -> String {
    p.shape.clone().unwrap_or_else(|| {
        if p.outline == Outline::Rounded {
            "rounded".into()
        } else {
            "rect".into()
        }
    })
}
