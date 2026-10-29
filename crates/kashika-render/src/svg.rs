use kashika_geom::{PathSeg, Pt};
use kashika_model::{
    Label, Layer, Line, Marker, Scene, Shape, ShapeKind, StrokeStyle, Style, Text,
};

use crate::shapes::{self, Defs};
use crate::text;

enum Overlay<'s> {
    Label(&'s Line, &'s Label),
    Text(&'s Text),
}

pub fn render_svg(scene: &Scene) -> String {
    let (w, h) = scene.size;
    let defs = Defs::new(scene, diagram_id(scene));
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{0}\" height=\"{1}\" viewBox=\"0 0 {0} {1}\">\n",
        n(w),
        n(h)
    );
    defs.write(&mut out);
    if let Some(bg) = &scene.background {
        out.push_str(&format!(
            "<rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>\n"
        ));
    }
    for s in scene.shapes.iter().filter(|s| s.layer == Layer::Under) {
        shape(&mut out, s, &defs);
    }
    for g in scene.shapes.iter().filter(|s| s.kind == ShapeKind::Group) {
        shape(&mut out, g, &defs);
    }
    for b in scene
        .shapes
        .iter()
        .filter(|s| s.kind != ShapeKind::Group && s.layer == Layer::Normal)
    {
        shape(&mut out, b, &defs);
    }
    for j in scene.junctions.iter().filter(|j| j.dot) {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#334155\"/>\n",
            n(j.at.x),
            n(j.at.y)
        ));
    }
    for (i, l) in scene.lines.iter().enumerate() {
        line(&mut out, l, i, scene.size, defs.id());
    }
    let mut overlays: Vec<((u32, u32), Overlay)> = Vec::new();
    for l in &scene.lines {
        for lab in &l.labels {
            overlays.push(((l.span.line, l.span.col), Overlay::Label(l, lab)));
        }
    }
    for t in &scene.texts {
        overlays.push(((t.span.line, t.span.col), Overlay::Text(t)));
    }
    overlays.sort_by_key(|(k, _)| *k);
    for (_, o) in overlays {
        match o {
            Overlay::Label(l, lab) => {
                text::lines(&mut out, &lab.lines, l.style.font_size, &l.style.color);
            }
            Overlay::Text(t) => text::lines(&mut out, &t.lines, t.size, &t.color),
        }
    }
    for s in scene.shapes.iter().filter(|s| s.layer == Layer::Over) {
        shape(&mut out, s, &defs);
    }
    out.push_str("</svg>\n");
    out
}

/// A hash of the scene for ids: deterministic, and different between diagrams so several
/// SVGs can be inlined in one HTML page.
fn diagram_id(scene: &Scene) -> String {
    let fnv = |h: u32, bytes: &[u8]| {
        bytes
            .iter()
            .fold(h, |h, &b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
    };
    let mut h = fnv(0x811c_9dc5, &serde_json::to_vec(scene).unwrap_or_default());
    for a in &scene.assets {
        h = fnv(h, &a.bytes);
    }
    format!("{h:08x}")
}

pub(crate) fn n(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    let r = if r.is_finite() { r } else { v };
    if r == 0.0 {
        "0".to_string()
    } else {
        format!("{r}")
    }
}

/// Escapes `s` for an attribute value, dropping characters XML 1.0 forbids.
pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' | '\r' => out.push(c),
            '\0'..='\x1f' | '\u{fffe}' | '\u{ffff}' => {}
            _ => out.push(c),
        }
    }
    out
}

pub(crate) fn stroke_attrs(style: &Style) -> String {
    let mut s = format!(
        "stroke=\"{}\" stroke-width=\"{}\"",
        style.stroke,
        n(style.stroke_width)
    );
    match style.stroke_style {
        StrokeStyle::Solid => {}
        StrokeStyle::Dashed => s.push_str(" stroke-dasharray=\"6 4\""),
        StrokeStyle::Dotted => s.push_str(" stroke-dasharray=\"1.5 3\" stroke-linecap=\"round\""),
    }
    s
}

fn shape(out: &mut String, s: &Shape, defs: &Defs) {
    shapes::body(out, s, defs);
    for item in &s.text {
        text::lines(out, &item.lines, item.size, &item.color);
    }
}

/// Shortens the end segments so the stroke stops at each marker's base.
fn trim(path: &mut [PathSeg], start: Marker, end: Marker) {
    let (a, b) = (start.length(), end.length());
    match path {
        [] => {}
        [only] => *only = only.trim(a, b),
        [first, .., last] => {
            *first = first.trim(a, 0.0);
            *last = last.trim(0.0, b);
        }
    }
}

pub(crate) fn path_data(path: &[PathSeg]) -> String {
    let mut d = String::new();
    if let Some(first) = path.first() {
        let p = first.from();
        d.push_str(&format!("M {} {}", n(p.x), n(p.y)));
    }
    for seg in path {
        match *seg {
            PathSeg::Line { to, .. } => d.push_str(&format!(" L {} {}", n(to.x), n(to.y))),
            PathSeg::Quad { ctrl, to, .. } => d.push_str(&format!(
                " Q {} {} {} {}",
                n(ctrl.x),
                n(ctrl.y),
                n(to.x),
                n(to.y)
            )),
            PathSeg::Cubic { c1, c2, to, .. } => d.push_str(&format!(
                " C {} {} {} {} {} {}",
                n(c1.x),
                n(c1.y),
                n(c2.x),
                n(c2.y),
                n(to.x),
                n(to.y)
            )),
        }
    }
    d
}

/// Opens a group masked so the line and its markers are cut out under its own labels,
/// leaving whatever is beneath visible. Returns whether a group was opened.
fn label_mask(out: &mut String, l: &Line, i: usize, (w, h): (f64, f64), id: &str) -> bool {
    if l.labels.is_empty() {
        return false;
    }
    let id = format!("kashika-{id}-mask-{i}");
    out.push_str(&format!(
        "<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"{0}\" height=\"{1}\"><rect width=\"{0}\" height=\"{1}\" fill=\"#fff\"/>",
        n(w),
        n(h)
    ));
    for lab in &l.labels {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#000\"/>",
            n(lab.rect.x),
            n(lab.rect.y),
            n(lab.rect.w),
            n(lab.rect.h)
        ));
    }
    out.push_str(&format!("</mask>\n<g mask=\"url(#{id})\">\n"));
    true
}

fn line(out: &mut String, l: &Line, i: usize, size: (f64, f64), id: &str) {
    let masked = label_mask(out, l, i, size, id);
    let mut path = l.path.clone();
    trim(&mut path, l.start, l.end);
    out.push_str(&format!(
        "<path d=\"{}\" fill=\"none\" stroke-linejoin=\"round\" {}/>\n",
        path_data(&path),
        stroke_attrs(&l.style)
    ));
    if let (Some(first), Some(last)) = (l.path.first(), l.path.last()) {
        marker(out, l.end, last.to(), last.end_dir(), &l.style);
        marker(
            out,
            l.start,
            first.from(),
            first.start_dir() * -1.0,
            &l.style,
        );
    }
    if masked {
        out.push_str("</g>\n");
    }
}

/// Draws a marker whose tip sits on `tip`, pointing along `dir`.
fn marker(out: &mut String, m: Marker, tip: Pt, dir: Pt, style: &Style) {
    let pts = |ps: &[Pt]| {
        ps.iter()
            .map(|p| format!("{},{}", n(p.x), n(p.y)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let outline = m.outline(tip, dir);
    match m {
        Marker::None => {}
        Marker::Arrow | Marker::Diamond => {
            out.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\"/>\n",
                pts(&outline),
                style.stroke
            ));
        }
        Marker::Open => {
            out.push_str(&format!(
                "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"/>\n",
                pts(&[outline[1], outline[0], outline[2]]),
                style.stroke,
                n(style.stroke_width)
            ));
        }
        Marker::Dot => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>\n",
                n(tip.x),
                n(tip.y),
                n(Marker::DOT_RADIUS),
                style.stroke
            ));
        }
    }
}
