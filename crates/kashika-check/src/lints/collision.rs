use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{inside_runs, point_at, polygons_overlap, rect_polygon, Rect};

use crate::ctx::{bbox, fmt_num, intersection, line_el, shape_el, text_el, text_id, Ctx};
use crate::text_box::{label_box, label_box_at, text_box, title_box};

struct Subject<'c> {
    el: Element,
    span: Span,
    what: String,
    rect: Rect,
    /// For a line label: its line (index into `ctx.lines`) and the label.
    label: Option<(usize, &'c kashika_model::Label)>,
}

/// `label-collision`: a line label or text intersects a box, a group title, another label or
/// text, or a line other than its own.
pub fn label_collision(ctx: &Ctx) -> Vec<Diagnostic> {
    let subjects = subjects(ctx);
    let mut out = Vec::new();
    for (i, s) in subjects.iter().enumerate() {
        let mut hits = obstacles(ctx, s.rect, s.label.map(|(li, _)| li));
        for o in &subjects[..i] {
            if intersection(s.rect, o.rect).is_some() {
                hits.push((o.what.clone(), o.el.clone()));
            }
        }
        for (other, el) in hits {
            let mut d = Diagnostic::warn(
                "label-collision",
                s.span,
                format!("{} collides with {other}", s.what),
            )
            .with_element(s.el.clone())
            .with_element(el)
            .with_fact("bbox", bbox(s.rect));
            if let Some((li, label)) = s.label {
                let clear = clear_positions(ctx, &subjects, i, li, label);
                d = d
                    .with_fact("clear_t", &clear)
                    .with_help(describe_clear(&ctx.lines[li].line.id, &clear));
            }
            out.push(d);
        }
    }
    out
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
        });
    }
    out
}

/// Boxes, group titles and lines (except line `own`) that `rect` intersects.
fn obstacles(ctx: &Ctx, rect: Rect, own: Option<usize>) -> Vec<(String, Element)> {
    let poly = rect_polygon(rect);
    let mut out = Vec::new();
    for s in ctx.boxes() {
        if polygons_overlap(&poly, &s.poly) {
            out.push((format!("box '{}'", s.shape.id), shape_el(s.shape)));
        }
    }
    for g in ctx.groups() {
        if title_box(g.shape).is_some_and(|t| intersection(rect, t).is_some()) {
            out.push((
                format!("the title of group '{}'", g.shape.id),
                shape_el(g.shape),
            ));
        }
    }
    for (i, l) in ctx.lines.iter().enumerate() {
        if own != Some(i)
            && l.pieces
                .iter()
                .any(|&(p, q, _)| !inside_runs(p, q, &poly).is_empty())
        {
            out.push((format!("line {}", l.line.id), line_el(l.line, None)));
        }
    }
    out
}

/// `t` values (steps of 0.05) where label `i` on line `li` would collide with nothing.
fn clear_positions(
    ctx: &Ctx,
    subjects: &[Subject],
    i: usize,
    li: usize,
    label: &kashika_model::Label,
) -> Vec<f64> {
    let l = &ctx.lines[li];
    (1..20)
        .map(|k| f64::from(k) / 20.0)
        .filter(|&t| {
            let r = label_box_at(label, point_at(&l.line.path, t));
            obstacles(ctx, r, Some(li)).is_empty()
                && subjects
                    .iter()
                    .enumerate()
                    .all(|(j, o)| j == i || intersection(r, o.rect).is_none())
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
