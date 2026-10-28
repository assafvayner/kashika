use kashika_diag::{did_you_mean, Diagnostic};
use kashika_syntax::ast::{Attr, Value};

use crate::style::{FontWeight, StrokeStyle, StyleProps};

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
                if let Some(s) = did_you_mean(key, allowed.iter().copied()) {
                    d = d.with_help(format!("did you mean `{s}`?"));
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

pub fn type_err(a: &Attr, expected: &str) -> Diagnostic {
    Diagnostic::error(
        "attr",
        a.span,
        format!("`{}` expects {expected}", a.key.name),
    )
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
        _ => {
            diags.push(type_err(a, "a color like `#1a73e8` or `none`"));
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
        if let Some(s) = did_you_mean(w, options.iter().copied()) {
            d = d.with_help(format!("did you mean `{s}`?"));
        }
    }
    diags.push(d);
    None
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
        p.stroke_style = word(x, &["solid", "dashed", "dotted"], diags).map(|w| match w {
            "dashed" => StrokeStyle::Dashed,
            "dotted" => StrokeStyle::Dotted,
            _ => StrokeStyle::Solid,
        });
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
        p.padding = num_in(x, Min::NonNegative, diags);
    }
    if let Some(x) = a.get("muted") {
        p.muted = color(x, diags);
    }
    p
}
