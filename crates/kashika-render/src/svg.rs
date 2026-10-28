use kashika_geom::{PathSeg, Pt};
use kashika_model::{Line, Marker, Scene, Shape, ShapeKind, StrokeStyle, Style};

use crate::text;

const ARROW_LEN: f64 = 10.0;
const DIAMOND_LEN: f64 = 12.0;
const MARKER_HALF_WIDTH: f64 = 4.0;

pub fn render_svg(scene: &Scene) -> String {
    let (w, h) = scene.size;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{0}\" height=\"{1}\" viewBox=\"0 0 {0} {1}\">\n",
        n(w),
        n(h)
    );
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    let mut groups: Vec<&Shape> = scene
        .shapes
        .iter()
        .filter(|s| s.kind == ShapeKind::Group)
        .collect();
    groups.sort_by_key(|s| s.depth);
    for g in groups {
        shape(&mut out, g);
    }
    for b in scene.shapes.iter().filter(|s| s.kind == ShapeKind::Box) {
        shape(&mut out, b);
    }
    for j in scene.junctions.iter().filter(|j| j.dot) {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#334155\"/>\n",
            n(j.at.x),
            n(j.at.y)
        ));
    }
    for l in &scene.lines {
        line(&mut out, l);
    }
    for l in &scene.lines {
        for lab in &l.labels {
            text::lines(
                &mut out,
                &lab.lines,
                l.style.font_size,
                &l.style.color,
                true,
            );
        }
    }
    for t in &scene.texts {
        text::lines(&mut out, &t.lines, t.size, &t.color, false);
    }
    out.push_str("</svg>\n");
    out
}

fn n(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    let r = if r.is_finite() { r } else { v };
    if r == 0.0 {
        "0".to_string()
    } else {
        format!("{r}")
    }
}

pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn stroke_attrs(style: &Style) -> String {
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

fn shape(out: &mut String, s: &Shape) {
    let r = s.rect;
    let st = &s.style;
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" {}/>\n",
        n(r.x),
        n(r.y),
        n(r.w),
        n(r.h),
        n(s.corner_radius()),
        st.fill,
        stroke_attrs(st)
    ));
    for item in &s.text {
        text::lines(out, &item.lines, item.size, &item.color, false);
    }
}

fn marker_len(m: Marker) -> f64 {
    match m {
        Marker::Arrow => ARROW_LEN,
        Marker::Diamond => DIAMOND_LEN,
        Marker::None | Marker::Open | Marker::Dot => 0.0,
    }
}

/// Shortens the end segments so the stroke stops at each marker's base.
fn trim(path: &mut [PathSeg], start: Marker, end: Marker) {
    let (a, b) = (marker_len(start), marker_len(end));
    match path {
        [] => {}
        [only] => *only = only.trim(a, b),
        [first, .., last] => {
            *first = first.trim(a, 0.0);
            *last = last.trim(0.0, b);
        }
    }
}

fn path_data(path: &[PathSeg]) -> String {
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

fn line(out: &mut String, l: &Line) {
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
}

/// Draws a marker whose tip sits on `tip`, pointing along `dir`.
fn marker(out: &mut String, m: Marker, tip: Pt, dir: Pt, style: &Style) {
    let u = dir.norm();
    let perp = Pt::new(-u.y, u.x) * MARKER_HALF_WIDTH;
    let pts = |ps: &[Pt]| {
        ps.iter()
            .map(|p| format!("{},{}", n(p.x), n(p.y)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    match m {
        Marker::None => {}
        Marker::Arrow => {
            let base = tip - u * ARROW_LEN;
            out.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\"/>\n",
                pts(&[tip, base + perp, base - perp]),
                style.stroke
            ));
        }
        Marker::Open => {
            let base = tip - u * ARROW_LEN;
            out.push_str(&format!(
                "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"/>\n",
                pts(&[base + perp, tip, base - perp]),
                style.stroke,
                n(style.stroke_width)
            ));
        }
        Marker::Diamond => {
            let mid = tip - u * (DIAMOND_LEN / 2.0);
            let back = tip - u * DIAMOND_LEN;
            out.push_str(&format!(
                "<polygon points=\"{}\" fill=\"{}\"/>\n",
                pts(&[tip, mid + perp, back, mid - perp]),
                style.stroke
            ));
        }
        Marker::Dot => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"{}\"/>\n",
                n(tip.x),
                n(tip.y),
                style.stroke
            ));
        }
    }
}
