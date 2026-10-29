use kashika_syntax::ast::Size;
use kashika_text::VAlign;
use serde::{Serialize, Serializer};

use crate::scene::Align;

pub const STYLE_KEYS: &[&str] = &[
    "fill",
    "stroke",
    "stroke-width",
    "stroke-style",
    "font-size",
    "font-weight",
    "color",
    "radius",
    "padding",
    "muted",
    "text-align",
    "text-valign",
];

/// Keys a `style` definition takes: the style keys, `shape` and `size`.
pub fn def_keys() -> Vec<&'static str> {
    STYLE_KEYS
        .iter()
        .copied()
        .chain(["shape", "size"])
        .collect()
}

/// A built-in style: colors only, so it sets no shape or size.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinStyle {
    pub name: &'static str,
    pub fill: &'static str,
    pub stroke: &'static str,
    pub stroke_style: Option<StrokeStyle>,
    /// The text color, when the style sets one.
    pub color: Option<&'static str>,
}

/// Every built-in style, in the order docs list them.
pub const STYLES: &[BuiltinStyle] = &[
    BuiltinStyle::colors("service", "#e8f0fe", "#1a73e8"),
    BuiltinStyle::colors("datastore", "#e6f4ea", "#137333"),
    BuiltinStyle::colors("external", "#f1f3f4", "#5f6368"),
    BuiltinStyle::colors("queue", "#fef7e0", "#b06000"),
    BuiltinStyle {
        name: "boundary",
        fill: "none",
        stroke: "#94a3b8",
        stroke_style: Some(StrokeStyle::Dashed),
        color: Some("#475569"),
    },
    BuiltinStyle::colors("note", "#fffbe6", "#d4b106"),
];

impl BuiltinStyle {
    const fn colors(name: &'static str, fill: &'static str, stroke: &'static str) -> Self {
        BuiltinStyle {
            name,
            fill,
            stroke,
            stroke_style: None,
            color: None,
        }
    }

    pub fn props(&self) -> StyleProps {
        StyleProps {
            fill: Some(self.fill.into()),
            stroke: Some(self.stroke.into()),
            stroke_style: self.stroke_style,
            color: self.color.map(Into::into),
            ..StyleProps::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

impl StrokeStyle {
    pub const NAMES: &'static [&'static str] = &["solid", "dashed", "dotted"];

    pub fn from_name(name: &str) -> Option<StrokeStyle> {
        [StrokeStyle::Solid, StrokeStyle::Dashed, StrokeStyle::Dotted]
            .into_iter()
            .find(|x| x.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            StrokeStyle::Solid => "solid",
            StrokeStyle::Dashed => "dashed",
            StrokeStyle::Dotted => "dotted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Normal,
    Bold,
}

/// The text area inset: `x` on the left and right, `y` on the top and bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub x: f64,
    pub y: f64,
}

impl Padding {
    pub fn uniform(v: f64) -> Self {
        Padding { x: v, y: v }
    }
}

/// One number when both axes match, otherwise `[x, y]`.
impl Serialize for Padding {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.x == self.y {
            s.serialize_f64(self.x)
        } else {
            (self.x, self.y).serialize(s)
        }
    }
}

/// What `currentColor` in an icon resolves to when neither the shape nor a style it names sets
/// `color` (a group's built-in `boundary` color doesn't count): the default stroke, which
/// bundled icons were drawn in.
pub const ICON_COLOR: &str = "#334155";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Style {
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub stroke_style: StrokeStyle,
    pub font_size: f64,
    pub font_weight: FontWeight,
    pub color: String,
    pub radius: f64,
    pub padding: Padding,
    pub muted: String,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fill: "#ffffff".into(),
            stroke: "#334155".into(),
            stroke_width: 1.5,
            stroke_style: StrokeStyle::Solid,
            font_size: 14.0,
            font_weight: FontWeight::Normal,
            color: "#0f172a".into(),
            radius: 0.0,
            padding: Padding::uniform(8.0),
            muted: "#64748b".into(),
        }
    }
}

/// A partial style; `None` means "inherit".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleProps {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_style: Option<StrokeStyle>,
    pub font_size: Option<f64>,
    pub font_weight: Option<FontWeight>,
    pub color: Option<String>,
    pub radius: Option<f64>,
    pub padding: Option<Padding>,
    pub muted: Option<String>,
    pub text_align: Option<Align>,
    /// Boxes only.
    pub text_valign: Option<VAlign>,
    /// The qualified shape name; boxes only.
    pub shape: Option<String>,
    /// Boxes and groups only.
    pub size: Option<Size>,
}

impl StyleProps {
    /// Returns `self` with every key that `top` sets replaced by `top`'s value.
    pub fn overlay(&self, top: &StyleProps) -> StyleProps {
        StyleProps {
            fill: top.fill.clone().or_else(|| self.fill.clone()),
            stroke: top.stroke.clone().or_else(|| self.stroke.clone()),
            stroke_width: top.stroke_width.or(self.stroke_width),
            stroke_style: top.stroke_style.or(self.stroke_style),
            font_size: top.font_size.or(self.font_size),
            font_weight: top.font_weight.or(self.font_weight),
            color: top.color.clone().or_else(|| self.color.clone()),
            radius: top.radius.or(self.radius),
            padding: top.padding.or(self.padding),
            muted: top.muted.clone().or_else(|| self.muted.clone()),
            text_align: top.text_align.or(self.text_align),
            text_valign: top.text_valign.or(self.text_valign),
            shape: top.shape.clone().or_else(|| self.shape.clone()),
            size: top.size.or(self.size),
        }
    }

    pub fn resolve(&self) -> Style {
        let d = Style::default();
        Style {
            fill: self.fill.clone().unwrap_or(d.fill),
            stroke: self.stroke.clone().unwrap_or(d.stroke),
            stroke_width: self.stroke_width.unwrap_or(d.stroke_width),
            stroke_style: self.stroke_style.unwrap_or(d.stroke_style),
            font_size: self.font_size.unwrap_or(d.font_size),
            font_weight: self.font_weight.unwrap_or(d.font_weight),
            color: self.color.clone().unwrap_or(d.color),
            radius: self.radius.unwrap_or(d.radius),
            padding: self.padding.unwrap_or(d.padding),
            muted: self.muted.clone().unwrap_or(d.muted),
        }
    }

    /// Only the keys that apply to lines.
    pub fn line_keys(&self) -> StyleProps {
        StyleProps {
            stroke: self.stroke.clone(),
            stroke_width: self.stroke_width,
            stroke_style: self.stroke_style,
            font_size: self.font_size,
            font_weight: self.font_weight,
            color: self.color.clone(),
            ..StyleProps::default()
        }
    }
}

pub fn builtin(name: &str) -> Option<StyleProps> {
    STYLES
        .iter()
        .find(|s| s.name == name)
        .map(BuiltinStyle::props)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_is_key_by_key() {
        let base = builtin("service").unwrap();
        let user = StyleProps {
            fill: Some("#ffffff".into()),
            ..StyleProps::default()
        };
        let s = base.overlay(&user).resolve();
        assert_eq!(s.fill, "#ffffff");
        assert_eq!(s.stroke, "#1a73e8");
        assert_eq!(s.stroke_width, 1.5);
    }
}
