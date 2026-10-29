use kashika_diag::{Diagnostic, did_you_mean};
use kashika_syntax::ast::{Attr, Size, Value};

use crate::colors;
use crate::style::{FontWeight, Padding, StrokeStyle, StyleProps};

/// Attribute names written where another name is meant: (written, meant). Each is suggested
/// only where the meant name is allowed and the written one isn't.
const MEANT: &[(&str, &str)] = &[
    ("size", "font-size"),
    ("text-size", "font-size"),
    ("align", "text-align"),
    ("valign", "text-valign"),
    ("vertical-align", "text-valign"),
    ("font-color", "color"),
    ("text-color", "color"),
    ("weight", "font-weight"),
];

/// Validated view over a statement's attributes: reports unknown keys (with suggestions),
/// duplicates, and `@t` on keys other than `label`.
pub struct Attrs<'a> {
    list: Vec<&'a Attr>,
}

impl<'a> Attrs<'a> {
    pub fn new(
        list: &'a [Attr],
        what: &str,
        allowed: &[&str],
        diags: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut seen: Vec<(&str, Option<u64>)> = Vec::new();
        let mut kept: Vec<&'a Attr> = Vec::new();
        for a in list {
            let key = a.key.name.as_str();
            if !allowed.contains(&key) {
                let mut d = Diagnostic::error(
                    "attr",
                    a.key.span,
                    format!("unknown attribute `{key}` on {what}"),
                );
                if let Some(h) = unknown_help(a, what, allowed) {
                    d = d.with_help(h);
                }
                diags.push(d);
                continue;
            }
            if a.at_t.is_some() && key != "label" {
                diags.push(Diagnostic::error(
                    "attr",
                    a.span,
                    format!("`@` positions only apply to `label`, not `{key}`"),
                ));
                continue;
            }
            let k = (key, a.at_t.map(f64::to_bits));
            if seen.contains(&k) {
                diags.push(Diagnostic::error(
                    "attr",
                    a.key.span,
                    format!("duplicate attribute `{key}`"),
                ));
                continue;
            }
            seen.push(k);
            kept.push(a);
        }
        Self { list: kept }
    }

    pub fn get(&self, key: &str) -> Option<&'a Attr> {
        self.list
            .iter()
            .copied()
            .find(|a| a.key.name == key && a.at_t.is_none())
    }

    pub fn all(&self, key: &'static str) -> impl Iterator<Item = &'a Attr> + '_ {
        self.list.iter().copied().filter(move |a| a.key.name == key)
    }
}

/// The help for an attribute `allowed` doesn't take: `size (w,h)` for a `size=` pair, else the
/// name meant in this context, else the nearest allowed name.
fn unknown_help(a: &Attr, what: &str, allowed: &[&str]) -> Option<String> {
    let key = a.key.name.as_str();
    if key == "size" && matches!(a.value, Value::Pair(..) | Value::Size(_)) {
        return Some(
            "`size` is a clause, not an attribute: write `size (w,h)`, without `=`".into(),
        );
    }
    if key == "label" && matches!(what, "box" | "group" | "text") {
        let text = match &a.value {
            Value::Str(s) => s.as_str(),
            _ => "…",
        };
        let id = if what == "text" { "" } else { "<id> " };
        return Some(format!(
            "the label is the quoted string after the id, with no `label=`: `{what} {id}\"{text}\" …`"
        ));
    }
    if matches!(key, "arrow" | "arrowhead" | "head") && allowed.contains(&"end") {
        let ends = match &a.value {
            Value::Word(w) if w == "start" => "`start=arrow`",
            Value::Word(w) if w == "both" => "`start=arrow end=arrow`",
            _ => "`end=arrow`",
        };
        return Some(format!(
            "arrowheads are line ends: write {ends}, or use the `arrow` statement"
        ));
    }
    if matches!(key, "width" | "height" | "w" | "h") && matches!(what, "box" | "group" | "image") {
        return Some("the size is a clause: write `size (w,h)`, without `=`".into());
    }
    if StrokeStyle::NAMES.contains(&key) && allowed.contains(&"stroke-style") {
        return Some(format!("write `stroke-style={key}`"));
    }
    MEANT
        .iter()
        .find(|(written, meant)| *written == key && allowed.contains(meant))
        .map(|(_, meant)| *meant)
        .or_else(|| did_you_mean(key, allowed.iter().copied()))
        .map(|s| format!("did you mean `{s}`?"))
}

/// The attribute set as `name` or as its `alias`. Setting both is a duplicate, reported at
/// the later one; the earlier one is kept.
pub fn aliased<'a>(
    a: &Attrs<'a>,
    name: &str,
    alias: &str,
    diags: &mut Vec<Diagnostic>,
) -> Option<&'a Attr> {
    match (a.get(name), a.get(alias)) {
        (Some(x), Some(y)) => {
            let (first, second) = if (y.span.line, y.span.col) < (x.span.line, x.span.col) {
                (y, x)
            } else {
                (x, y)
            };
            diags.push(
                Diagnostic::error(
                    "attr",
                    second.key.span,
                    format!("duplicate attribute: `{alias}` is another name for `{name}`"),
                )
                .with_help(format!(
                    "keep one of `{}` and `{}`",
                    first.key.name, second.key.name
                )),
            );
            Some(first)
        }
        (x, y) => x.or(y),
    }
}

pub fn type_err(a: &Attr, expected: &str) -> Diagnostic {
    let d = Diagnostic::error(
        "attr",
        a.span,
        format!("`{}` expects {expected}", a.key.name),
    );
    match &a.value {
        Value::Word(w) if expected == "a string" => {
            d.with_help(format!("quote it: `{}=\"{w}\"`", a.key.name))
        }
        _ => d,
    }
}

pub fn num(a: &Attr, diags: &mut Vec<Diagnostic>) -> Option<f64> {
    if let Value::Num(n) = a.value {
        Some(n)
    } else {
        diags.push(type_err(a, "a number"));
        None
    }
}

/// Largest magnitude allowed for any coordinate, size or numeric attribute.
pub const LIMIT: f64 = 1e6;

#[derive(Clone, Copy)]
pub enum Min {
    Positive,
    NonNegative,
}

/// A number in `min..=LIMIT`; anything else is an `attr` error.
pub fn num_in(a: &Attr, min: Min, diags: &mut Vec<Diagnostic>) -> Option<f64> {
    let v = num(a, diags)?;
    let low = match min {
        Min::Positive if v <= 0.0 => Some("greater than 0"),
        Min::NonNegative if v < 0.0 => Some("at least 0"),
        _ => None,
    };
    let msg = match low {
        Some(bound) => format!("`{}` must be {bound}", a.key.name),
        None if v > LIMIT => format!("`{}` must be at most {LIMIT}", a.key.name),
        None => return Some(v),
    };
    diags.push(Diagnostic::error("attr", a.span, msg));
    None
}

/// `padding=N` or `padding=(x,y)`, each value in `0..=LIMIT`.
fn padding(a: &Attr, diags: &mut Vec<Diagnostic>) -> Option<Padding> {
    match a.value {
        Value::Num(_) => num_in(a, Min::NonNegative, diags).map(Padding::uniform),
        Value::Pair(x, y) => {
            let bad = |v: f64| !(0.0..=LIMIT).contains(&v);
            if bad(x) || bad(y) {
                diags.push(
                    Diagnostic::error(
                        "attr",
                        a.span,
                        format!("`padding` values must be at least 0 and at most {LIMIT}"),
                    )
                    .with_help("write `padding=(x,y)` with x and y between 0 and 1000000"),
                );
                return None;
            }
            Some(Padding { x, y })
        }
        _ => {
            diags.push(
                type_err(a, "a number or a pair `(x,y)`")
                    .with_help("write `padding=8` for every side, or `padding=(x,y)` for left/right and top/bottom"),
            );
            None
        }
    }
}

/// A style's `size=(w,h)`: each axis a number in `0 < v <= LIMIT`, or `auto`.
pub fn size(a: &Attr, diags: &mut Vec<Diagnostic>) -> Option<Size> {
    match a.value {
        Value::Size(s) => {
            let bad = |v: Option<f64>| v.is_some_and(|v| !(v > 0.0 && v <= LIMIT));
            if bad(s.w) || bad(s.h) {
                diags.push(Diagnostic::error(
                    "size",
                    a.span,
                    format!("`size` must be positive and at most {LIMIT}"),
                ));
                return None;
            }
            Some(s)
        }
        Value::Num(_) => {
            diags.push(
                type_err(a, "a size `(w,h)`").with_help("for the text size, write `font-size=N`"),
            );
            None
        }
        _ => {
            diags.push(type_err(a, "a size `(w,h)`"));
            None
        }
    }
}

pub fn text(a: &Attr, diags: &mut Vec<Diagnostic>) -> Option<String> {
    if let Value::Str(s) = &a.value {
        Some(s.clone())
    } else {
        diags.push(type_err(a, "a string"));
        None
    }
}

pub fn color(a: &Attr, diags: &mut Vec<Diagnostic>) -> Option<String> {
    match &a.value {
        Value::Color(c) => Some(c.clone()),
        Value::Word(w) if w == "none" => Some(w.clone()),
        Value::Word(w) => {
            if let Some(hex) = colors::hex(w) {
                return Some(hex.to_string());
            }
            let mut d = type_err(a, "a color like `#1a73e8`, a CSS color name or `none`");
            if let Some(s) = colors::suggest(w) {
                d = d.with_help(format!("did you mean `{s}`?"));
            }
            diags.push(d);
            None
        }
        _ => {
            diags.push(type_err(
                a,
                "a color like `#1a73e8`, a CSS color name or `none`",
            ));
            None
        }
    }
}

pub fn flag(a: &Attr, diags: &mut Vec<Diagnostic>) -> bool {
    if a.value == Value::Flag {
        true
    } else {
        diags.push(Diagnostic::error(
            "attr",
            a.span,
            format!("`{}` takes no value", a.key.name),
        ));
        false
    }
}

pub fn word(
    a: &Attr,
    options: &[&'static str],
    diags: &mut Vec<Diagnostic>,
) -> Option<&'static str> {
    if let Value::Word(w) = &a.value {
        if let Some(o) = options.iter().find(|o| **o == w.as_str()) {
            return Some(*o);
        }
    }
    let mut d = Diagnostic::error(
        "attr",
        a.span,
        format!("`{}` expects one of: {}", a.key.name, options.join(", ")),
    );
    if let Value::Word(w) = &a.value {
        if let Some(s) = suggest_word(w, options) {
            d = d.with_help(format!("did you mean `{s}`?"));
        }
    }
    diags.push(d);
    None
}

/// Words written where one of `options` is meant: (written, meant), checked before spelling.
const WORDS_MEANT: &[(&str, &str)] = &[
    ("middle", "center"),
    ("mid", "center"),
    ("centre", "center"),
    ("north", "top"),
    ("south", "bottom"),
    ("east", "right"),
    ("west", "left"),
    ("arrowhead", "arrow"),
];

/// The option meant by `w`: a known synonym, else the nearest spelling.
pub fn suggest_word<'a>(w: &str, options: &[&'a str]) -> Option<&'a str> {
    WORDS_MEANT
        .iter()
        .find(|(written, meant)| *written == w && options.contains(meant))
        .and_then(|(_, meant)| options.iter().copied().find(|o| o == meant))
        .or_else(|| did_you_mean(w, options.iter().copied()))
}

pub fn style_props(a: &Attrs, diags: &mut Vec<Diagnostic>) -> StyleProps {
    let mut p = StyleProps::default();
    if let Some(x) = a.get("fill") {
        p.fill = color(x, diags);
    }
    if let Some(x) = a.get("stroke") {
        p.stroke = color(x, diags);
    }
    if let Some(x) = a.get("stroke-width") {
        p.stroke_width = num_in(x, Min::Positive, diags);
    }
    if let Some(x) = a.get("stroke-style") {
        p.stroke_style = word(x, StrokeStyle::NAMES, diags).and_then(StrokeStyle::from_name);
    }
    if let Some(x) = a.get("font-size") {
        p.font_size = num_in(x, Min::Positive, diags);
    }
    if let Some(x) = a.get("font-weight") {
        p.font_weight = word(x, &["normal", "bold"], diags).map(|w| {
            if w == "bold" {
                FontWeight::Bold
            } else {
                FontWeight::Normal
            }
        });
    }
    if let Some(x) = a.get("color") {
        p.color = color(x, diags);
    }
    if let Some(x) = a.get("radius") {
        p.radius = num_in(x, Min::NonNegative, diags);
    }
    if let Some(x) = a.get("padding") {
        p.padding = padding(x, diags);
    }
    if let Some(x) = a.get("muted") {
        p.muted = color(x, diags);
    }
    p
}
