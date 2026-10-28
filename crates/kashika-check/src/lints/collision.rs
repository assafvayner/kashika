use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{Pt, Rect, inside_runs, point_at, polygons_overlap, rect_polygon};
use kashika_model::Marker;

use crate::ctx::{
    Ctx, bbox, fmt_num, intersection, kind_name, line_el, shape_el, text_el, text_id,
};
use crate::index::Grid;
use crate::sink::Sink;
use crate::text_box::{below_box, label_box, label_box_at, text_box, title_box};

const ARROW_LEN: f64 = 10.0;
const DIAMOND_LEN: f64 = 12.0;
const MARKER_HALF_WIDTH: f64 = 4.0;
const DOT_RADIUS: f64 = 3.0;

struct Subject<'c> {
    el: Element,
    span: Span,
    what: String,
    rect: Rect,
    /// For a line label: its line (index into `ctx.lines`) and the label.
    label: Option<(usize, &'c kashika_model::Label)>,
    /// For the label under a `text-area=below` shape: that shape's id.
    shape: Option<&'c str>,
}

/// `label-collision`: a line label, text or the label under a `text-area=below` shape
/// intersects a box, an image, a group title, another label or text, a line other than
/// its own, or (for a line label) its own line's markers.
pub fn label_collision(ctx: &Ctx, out: &mut Sink) {
    let subjects = subjects(ctx);
    let grid = Grid::new(&subjects.iter().map(|s| s.rect).collect::<Vec<_>>());
    for (i, s) in subjects.iter().enumerate() {
        let mut hits = obstacles(ctx, s.rect, s.label.map(|(li, _)| li), s.shape);
        for o in grid
            .near(s.rect, 1.0)
            .into_iter()
            .take_while(|&j| j < i)
            .map(|j| &subjects[j])
        {
            if intersection(s.rect, o.rect).is_some() {
                hits.push((o.what.clone(), o.el.clone()));
            }
        }
        if hits.is_empty() {
            continue;
        }
        let clear = s.label.map(|(li, label)| {
            let ts = clear_positions(ctx, &subjects, &grid, i, li, label);
            let help = describe_clear(&ctx.lines[li].line.id, &ts);
            (ts, help)
        });
        for (other, el) in hits {
            let mut d = Diagnostic::warn(
                "label-collision",
                s.span,
                format!("{} collides with {other}", s.what),
            )
            .with_element(s.el.clone())
            .with_element(el)
            .with_fact("bbox", bbox(s.rect));
            if let Some((ts, help)) = &clear {
                d = d.with_fact("clear_t", ts).with_help(help);
            }
            out.push(d);
        }
    }
}

fn subjects<'c>(ctx: &'c Ctx) -> Vec<Subject<'c>> {
    let mut out = Vec::new();
    for (li, l) in ctx.lines.iter().enumerate() {
        for lab in &l.line.labels {
            let rect = label_box(lab);
            let mut el = Element::new(l.line.id.clone(), "label", lab.span);
            el.bbox = Some(bbox(rect));
            out.push(Subject {
                el,
                span: lab.span,
                what: format!("label \"{}\" of {}", lab.text, l.line.id),
                rect,
                label: Some((li, lab)),
                shape: None,
            });
        }
    }
    for t in &ctx.scene.texts {
        out.push(Subject {
            el: text_el(t),
            span: t.span,
            what: format!("text '{}'", text_id(t)),
            rect: text_box(t),
            label: None,
            shape: None,
        });
    }
    for s in ctx.boxes() {
        let Some(rect) = below_box(s.shape) else {
            continue;
        };
        let mut el = Element::new(s.shape.id.clone(), "label", s.shape.span);
        el.bbox = Some(bbox(rect));
        out.push(Subject {
            el,
            span: s.shape.span,
            what: format!("the label under {} '{}'", kind_name(s.shape), s.shape.id),
            rect,
            label: None,
            shape: Some(s.shape.id.as_str()),
        });
    }
    out
}

/// Boxes, images, group titles, lines (except line `own` and shape `own_shape`) and the
/// markers of line `own` that `rect` intersects.
fn obstacles(
    ctx: &Ctx,
    rect: Rect,
    own: Option<usize>,
    own_shape: Option<&str>,
) -> Vec<(String, Element)> {
    let poly = rect_polygon(rect);
    let mut out = Vec::new();
    for s in ctx.boxes_near(rect, 0.0) {
        if own_shape != Some(s.shape.id.as_str()) && polygons_overlap(&poly, &s.poly) {
            out.push((
                format!("{} '{}'", kind_name(s.shape), s.shape.id),
                shape_el(s.shape),
            ));
        }
    }
    for g in ctx.groups_near(rect, 0.0) {
        if title_box(g.shape).is_some_and(|t| intersection(rect, t).is_some()) {
            out.push((
                format!("the title of group '{}'", g.shape.id),
                shape_el(g.shape),
            ));
        }
    }
    for i in ctx.lines_near(rect, 0.0) {
        let l = &ctx.lines[i];
        if own != Some(i)
            && l.pieces
                .iter()
                .any(|&(p, q, _)| !inside_runs(p, q, &poly).is_empty())
        {
            out.push((format!("line {}", l.line.id), line_el(l.line, None)));
        }
    }
    if let Some(l) = own.map(|i| &ctx.lines[i]) {
        for (end, name, marker) in markers(l.line) {
            if polygons_overlap(&poly, &marker) {
                let seg = if end == "start" { 0 } else { l.last_segment() };
                out.push((
                    format!("the {end} {name} of {}", l.line.id),
                    line_el(l.line, Some(seg)),
                ));
            }
        }
    }
    out
}

/// The outlines of a line's start and end markers, as `kashika-render` draws them:
/// (end, marker name, polygon).
fn markers(l: &kashika_model::Line) -> Vec<(&'static str, &'static str, Vec<Pt>)> {
    let (Some(first), Some(last)) = (l.path.first(), l.path.last()) else {
        return Vec::new();
    };
    [
        ("start", l.start, first.from(), first.start_dir() * -1.0),
        ("end", l.end, last.to(), last.end_dir()),
    ]
    .into_iter()
    .filter_map(|(end, m, tip, dir)| {
        let u = dir.norm();
        let perp = Pt::new(-u.y, u.x) * MARKER_HALF_WIDTH;
        let (name, poly) = match m {
            Marker::None => return None,
            Marker::Arrow | Marker::Open => {
                let base = tip - u * ARROW_LEN;
                ("arrowhead", vec![tip, base + perp, base - perp])
            }
            Marker::Diamond => {
                let mid = tip - u * (DIAMOND_LEN / 2.0);
                (
                    "diamond",
                    vec![tip, mid + perp, tip - u * DIAMOND_LEN, mid - perp],
                )
            }
            Marker::Dot => (
                "dot",
                rect_polygon(Rect::new(
                    tip.x - DOT_RADIUS,
                    tip.y - DOT_RADIUS,
                    2.0 * DOT_RADIUS,
                    2.0 * DOT_RADIUS,
                )),
            ),
        };
        Some((end, name, poly))
    })
    .collect()
}

/// `t` values (steps of 0.05) where label `i` on line `li` would collide with nothing.
fn clear_positions(
    ctx: &Ctx,
    subjects: &[Subject],
    grid: &Grid,
    i: usize,
    li: usize,
    label: &kashika_model::Label,
) -> Vec<f64> {
    let l = &ctx.lines[li];
    (1..20)
        .map(|k| f64::from(k) / 20.0)
        .filter(|&t| {
            let r = label_box_at(label, point_at(&l.line.path, t));
            obstacles(ctx, r, Some(li), None).is_empty()
                && grid
                    .near(r, 1.0)
                    .into_iter()
                    .all(|j| j == i || intersection(r, subjects[j].rect).is_none())
        })
        .collect()
}

fn describe_clear(line: &str, ts: &[f64]) -> String {
    if ts.is_empty() {
        return format!("no clear label position on {line}; move the line or the obstacle");
    }
    let mut ranges: Vec<(f64, f64)> = Vec::new();
    for &t in ts {
        match ranges.last_mut() {
            Some(r) if (t - r.1 - 0.05).abs() < 1e-9 => r.1 = t,
            _ => ranges.push((t, t)),
        }
    }
    let parts: Vec<String> = ranges
        .iter()
        .map(|&(a, b)| {
            if a == b {
                format!("label@{}", fmt_num(a))
            } else {
                format!("label@{}..{}", fmt_num(a), fmt_num(b))
            }
        })
        .collect();
    format!("clear label positions on {line}: {}", parts.join(", "))
}
