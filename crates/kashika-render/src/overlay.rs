use kashika_diag::{Diagnostic, Severity, Span};
use kashika_geom::{point_at, PathSeg, Pt, Rect};
use kashika_model::{Font, Run, Scene};

use crate::svg::{esc, n, path_data, render_svg};

const ERROR: &str = "#dc2626";
const WARN: &str = "#ea580c";
const GUIDE: &str = "#2563eb";
const RULER: &str = "#94a3b8";
const ANCHORS: [&str; 9] = [
    "top", "bottom", "left", "right", "center", "tl", "tr", "bl", "br",
];

/// What to draw over the diagram. The default draws nothing, leaving the SVG unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Overlay {
    /// Red markers at each error (`render --force`).
    pub error_markers: bool,
    /// Rulers, ids, anchor dots and markers for every diagnostic (`render --debug`).
    pub debug: bool,
}

pub fn render_svg_with(scene: &Scene, diags: &[Diagnostic], o: Overlay) -> String {
    let mut svg = render_svg(scene);
    if !o.error_markers && !o.debug {
        return svg;
    }
    let end = svg
        .rfind("</svg>")
        .expect("render_svg closes the root element");
    svg.truncate(end);
    svg.push_str("<g id=\"kashika-overlay\">\n");
    if o.debug {
        rulers(&mut svg, scene.size);
        guides(&mut svg, scene);
    }
    let mut unplaced = 0;
    for d in diags
        .iter()
        .filter(|d| o.debug || d.severity == Severity::Error)
    {
        match locate(scene, d) {
            Some(m) => mark(&mut svg, d, &m),
            None => {
                unplaced += 1;
                let text = format!("{}:{} {}", d.span.line, d.span.col, tag(d));
                label(
                    &mut svg,
                    &text,
                    Pt::new(4.0, 12.0 * f64::from(unplaced)),
                    color(d),
                );
            }
        }
    }
    svg.push_str("</g>\n</svg>\n");
    svg
}

enum Mark {
    Point(Pt),
    Rect(Rect),
    Path(Vec<PathSeg>),
}

fn color(d: &Diagnostic) -> &'static str {
    match d.severity {
        Severity::Error => ERROR,
        Severity::Warn => WARN,
    }
}

fn tag(d: &Diagnostic) -> String {
    let sev = match d.severity {
        Severity::Error => "error",
        Severity::Warn => "warn",
    };
    format!("{sev}[{}]", d.code)
}

fn locate(scene: &Scene, d: &Diagnostic) -> Option<Mark> {
    from_json(d).or_else(|| from_span(scene, d.span))
}

/// Geometry the diagnostic carries itself, under the JSON field names of spec §8.
fn from_json(d: &Diagnostic) -> Option<Mark> {
    let v = serde_json::to_value(d).ok()?;
    let pt = |v: &serde_json::Value| -> Option<Pt> {
        Some(Pt {
            x: v.get(0)?.as_f64()?,
            y: v.get(1)?.as_f64()?,
        })
    };
    for key in ["/facts/at", "/facts/entry"] {
        if let Some(p) = v.pointer(key).and_then(pt) {
            return Some(Mark::Point(p));
        }
    }
    let e = v.pointer("/elements/0")?;
    if let (Some(from), Some(to)) = (e.get("from").and_then(pt), e.get("to").and_then(pt)) {
        return Some(Mark::Path(vec![PathSeg::Line { from, to }]));
    }
    let bb = e.get("bbox")?;
    let (a, b) = (pt(bb.get(0)?)?, pt(bb.get(1)?)?);
    Some(Mark::Rect(Rect {
        x: a.x,
        y: a.y,
        w: b.x - a.x,
        h: b.y - a.y,
    }))
}

fn contains(outer: Span, inner: Span) -> bool {
    (outer.line, outer.col) <= (inner.line, inner.col)
        && (inner.line, inner.col) < (outer.end_line, outer.end_col)
}

/// The innermost scene element whose statement contains the diagnostic's start.
fn from_span(scene: &Scene, span: Span) -> Option<Mark> {
    let mut best: Option<(Span, Mark)> = None;
    let mut pick = |s: Span, m: Mark| {
        let later = match &best {
            Some((b, _)) => (s.line, s.col) > (b.line, b.col),
            None => true,
        };
        if contains(s, span) && later {
            best = Some((s, m));
        }
    };
    for s in &scene.shapes {
        pick(s.span, Mark::Rect(s.rect));
    }
    for l in &scene.lines {
        pick(l.span, Mark::Path(l.path.clone()));
    }
    for j in &scene.junctions {
        pick(j.span, Mark::Point(j.at));
    }
    for t in &scene.texts {
        pick(t.span, Mark::Point(t.at));
    }
    best.map(|(_, m)| m)
}

fn mark(out: &mut String, d: &Diagnostic, m: &Mark) {
    let c = color(d);
    let at = match m {
        Mark::Point(p) => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"{c}\" stroke-width=\"2\"/>\n",
                n(p.x),
                n(p.y)
            ));
            *p + Pt { x: 8.0, y: -8.0 }
        }
        Mark::Rect(r) => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{c}\" stroke-width=\"2\" stroke-dasharray=\"4 3\"/>\n",
                n(r.x - 3.0),
                n(r.y - 3.0),
                n(r.w + 6.0),
                n(r.h + 6.0)
            ));
            Pt {
                x: r.x - 3.0,
                y: r.y - 6.0,
            }
        }
        Mark::Path(p) => {
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"none\" stroke=\"{c}\" stroke-opacity=\"0.5\" stroke-width=\"6\"/>\n",
                path_data(p)
            ));
            point_at(p, 0.5) + Pt { x: 6.0, y: -6.0 }
        }
    };
    label(out, &tag(d), at, c);
}

/// Draws `text` as glyph outlines, so the overlay needs no fonts either.
fn label(out: &mut String, text: &str, at: Pt, color: &str) {
    let runs = [Run {
        text: text.to_string(),
        font: Font::Mono,
    }];
    let d = kashika_text::outline(&runs, 10.0, at.x, at.y);
    if d.is_empty() {
        return;
    }
    out.push_str(&format!(
        "<path aria-label=\"{}\" d=\"{d}\" fill=\"{color}\" stroke=\"#ffffff\" stroke-width=\"3\" stroke-linejoin=\"round\" paint-order=\"stroke\"/>\n",
        esc(text)
    ));
}

fn tick(v: f64) -> f64 {
    if v % 100.0 == 0.0 {
        10.0
    } else if v % 50.0 == 0.0 {
        6.0
    } else {
        3.0
    }
}

fn rulers(out: &mut String, (w, h): (f64, f64)) {
    let mut d = Vec::new();
    for i in 0..=(w / 10.0) as i64 {
        let x = i as f64 * 10.0;
        d.push(format!("M {} 0 V {}", n(x), n(tick(x))));
    }
    for i in 0..=(h / 10.0) as i64 {
        let y = i as f64 * 10.0;
        d.push(format!("M 0 {} H {}", n(y), n(tick(y))));
    }
    out.push_str(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{RULER}\" stroke-width=\"1\"/>\n",
        d.join(" ")
    ));
    for i in 1..=(w / 100.0) as i64 {
        let x = i as f64 * 100.0;
        label(
            out,
            &n(x),
            Pt {
                x: x + 2.0,
                y: 20.0,
            },
            RULER,
        );
    }
    for i in 1..=(h / 100.0) as i64 {
        let y = i as f64 * 100.0;
        label(
            out,
            &n(y),
            Pt {
                x: 12.0,
                y: y - 2.0,
            },
            RULER,
        );
    }
}

fn dot(out: &mut String, p: Pt) {
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"{GUIDE}\"/>\n",
        n(p.x),
        n(p.y)
    ));
}

fn guides(out: &mut String, scene: &Scene) {
    for s in &scene.shapes {
        label(
            out,
            &s.id,
            Pt {
                x: s.rect.x,
                y: s.rect.y - 3.0,
            },
            GUIDE,
        );
        for a in ANCHORS {
            if let Some(p) = s.anchor(a) {
                dot(out, p);
            }
        }
    }
    for l in &scene.lines {
        if !l.path.is_empty() {
            label(
                out,
                &l.id,
                point_at(&l.path, 0.5) + Pt { x: 4.0, y: 12.0 },
                GUIDE,
            );
        }
    }
    for j in &scene.junctions {
        dot(out, j.at);
        label(out, &j.id, j.at + Pt { x: 4.0, y: -4.0 }, GUIDE);
    }
}
