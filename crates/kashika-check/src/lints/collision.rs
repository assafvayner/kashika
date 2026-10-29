use kashika_diag::{Diagnostic, Element, Span, fact};
use kashika_geom::{
    PathSeg, Pt, Rect, fmt_num, inside_runs, point_at, polygons_overlap, rect_polygon,
};
use kashika_model::Marker;

use crate::ctx::{Ctx, arr, bbox, kind_name, line_el, shape_el, text_el, text_id};
use crate::index::Grid;
use crate::sink::Sink;
use crate::text_box::{below_box, label_box, label_box_at, text_box, title_band, title_box};

struct Subject<'c> {
    el: Element,
    span: Span,
    what: String,
    rect: Rect,
    /// For a line label: its line (index into `ctx.lines`) and the label.
    label: Option<(usize, &'c kashika_model::Label)>,
    /// For the label under a `text-area=below` shape: that shape's id.
    shape: Option<&'c str>,
    /// For a free `text`: the text.
    text: Option<&'c kashika_model::Text>,
}

/// `label-collision`: a line label, text or the label under a `text-area=below` shape
/// intersects a box, an image, a group title or border, another label or text, a line other
/// than its own, or (for a line label) its own line's markers; or a line crosses a group title.
pub fn label_collision(ctx: &Ctx, out: &mut Sink) {
    let subjects = subjects(ctx);
    let grid = Grid::new(&subjects.iter().map(|s| s.rect).collect::<Vec<_>>());
    for (i, s) in subjects.iter().enumerate() {
        let mut hits = obstacles(
            ctx,
            s.rect,
            s.label.map(|(li, lab)| (li, !lab.side.is_on())),
            s.shape,
        );
        for o in grid
            .near(s.rect, 1.0)
            .into_iter()
            .take_while(|&j| j < i)
            .map(|j| &subjects[j])
        {
            if s.rect.intersect(o.rect).is_some() {
                hits.push((o.what.clone(), o.el.clone()));
            }
        }
        if hits.is_empty() {
            continue;
        }
        let shape_hit = hits
            .iter()
            .any(|(_, el)| matches!(el.kind, "box" | "image"));
        let clear = s.label.map(|(li, label)| {
            let ts = clear_positions(ctx, &subjects, &grid, i, li, label);
            let help = if ts.is_empty() {
                no_clear_position(ctx, ctx.lines[li].line, label, shape_hit)
            } else {
                describe_clear(&ctx.lines[li].line.id, &ts)
            };
            (ts, help)
        });
        let into_block = s.text.and_then(|t| text_in_box(ctx, t));
        for (other, el) in hits {
            let mut d = Diagnostic::warn(
                "label-collision",
                s.span,
                format!("{} collides with {other}", s.what),
            )
            .with_element(s.el.clone())
            .with_element(el.clone())
            .with_fact("bbox", bbox(s.rect));
            if let Some((ts, help)) = &clear {
                d = d.with_fact("clear_t", ts).with_help(help);
            }
            if let Some((id, help)) = &into_block {
                if el.id == *id {
                    d = d.with_help(help);
                }
            }
            out.push(d);
        }
    }
    titles_crossed(ctx, out);
}

/// Lines that cross a group's title text.
fn titles_crossed(ctx: &Ctx, out: &mut Sink) {
    for g in ctx.groups() {
        let Some(t) = title_box(g.shape) else {
            continue;
        };
        let poly = rect_polygon(t);
        for i in ctx.lines_near(t, 0.0) {
            let l = &ctx.lines[i];
            let hit = l
                .pieces
                .iter()
                .find_map(|&(p, q, k)| inside_runs(p, q, &poly).first().map(|r| (r.0, k)));
            let Some((entry, k)) = hit else { continue };
            let seg = l.segment(k);
            let span = l.line.segments.get(seg).map_or(l.line.span, |s| s.span);
            let band = title_band(g.shape).map_or(t.bottom(), |b| b.bottom());
            let side = if entry.x < g.shape.rect.center().x {
                "right"
            } else {
                "left"
            };
            out.push(
                Diagnostic::warn(
                    "label-collision",
                    span,
                    format!(
                        "{} crosses the title of group '{}'",
                        l.line.id, g.shape.id
                    ),
                )
                .with_element(line_el(l.line, Some(seg)))
                .with_element(shape_el(g.shape))
                .with_fact("bbox", bbox(t))
                .with_fact(fact::ENTRY, arr(entry))
                .with_help(format!(
                    "the title of '{}' covers x {}..{}, y {}..{}; route {} around it, e.g. inside the group below y={} or outside it",
                    g.shape.id,
                    fmt_num(t.x),
                    fmt_num(t.right()),
                    fmt_num(t.y),
                    fmt_num(t.bottom()),
                    l.line.id,
                    fmt_num(band)
                ))
                .with_help(format!(
                    "or move the title away from the line: `text-align={side}` on '{}'",
                    g.shape.id
                )),
            );
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
                text: None,
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
            text: Some(t),
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
            text: None,
        });
    }
    out
}

/// Boxes, images, group titles, lines (except shape `own_shape`, and line `own` unless its
/// label sits beside it) and the markers of line `own` that `rect` intersects. `own` is the
/// line's index and whether the label sits beside the line rather than on it.
fn obstacles(
    ctx: &Ctx,
    rect: Rect,
    own: Option<(usize, bool)>,
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
        if title_box(g.shape).is_some_and(|t| rect.intersect(t).is_some()) {
            out.push((
                format!("the title of group '{}'", g.shape.id),
                shape_el(g.shape),
            ));
        }
        if rect.intersect(g.shape.rect).is_some() && !g.shape.rect.contains_rect(rect) {
            out.push((
                format!("the border of group '{}'", g.shape.id),
                shape_el(g.shape),
            ));
        }
    }
    for i in ctx.lines_near(rect, 0.0) {
        let l = &ctx.lines[i];
        if own != Some((i, false))
            && l.pieces
                .iter()
                .any(|&(p, q, _)| !inside_runs(p, q, &poly).is_empty())
        {
            out.push((format!("line {}", l.line.id), line_el(l.line, None)));
        }
    }
    if let Some(l) = own.map(|(i, _)| &ctx.lines[i]) {
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
    let w = l.style.stroke_width;
    [
        ("start", l.start, first.from(), first.start_dir() * -1.0),
        ("end", l.end, last.to(), last.end_dir()),
    ]
    .into_iter()
    .filter_map(|(end, m, tip, dir)| {
        let er = || l.er_parts(end == "start").hull();
        let (name, poly) = match m {
            Marker::None => return None,
            Marker::Arrow | Marker::Open => ("arrowhead", m.outline(tip, dir, w)),
            Marker::Triangle => ("triangle", m.outline(tip, dir, w)),
            Marker::Diamond | Marker::HollowDiamond => ("diamond", m.outline(tip, dir, w)),
            Marker::Dot => ("dot", m.outline(tip, dir, w)),
            Marker::Circle => ("circle", m.outline(tip, dir, w)),
            Marker::One => ("bar", er()),
            Marker::ZeroOne => ("bar and circle", er()),
            Marker::Many | Marker::OneMany | Marker::ZeroMany => ("crow's foot", er()),
        };
        Some((end, name, poly))
    })
    .collect()
}

/// `t` values (steps of 0.05) where label `i` on line `li` would collide with nothing and,
/// for a label beside its line, where that side fits the segment.
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
            if label.side.fits(&l.line.path, t).is_err() {
                return false;
            }
            let r = label_box_at(label, point_at(&l.line.path, t));
            obstacles(ctx, r, Some((li, !label.side.is_on())), None).is_empty()
                && grid
                    .near(r, 1.0)
                    .into_iter()
                    .all(|j| j == i || r.intersect(subjects[j].rect).is_none())
        })
        .collect()
}

/// For a free text whose anchor point is inside a box: the box's id, and the help to move the
/// text into that box's block, in box-local coordinates.
fn text_in_box(ctx: &Ctx, t: &kashika_model::Text) -> Option<(String, String)> {
    let at = Rect::new(t.at.x, t.at.y, 0.0, 0.0);
    let s = ctx.boxes_near(at, 0.0).find(|s| {
        s.shape.kind == kashika_model::ShapeKind::Box && s.shape.rect.contains_rect(at)
    })?;
    let r = s.shape.rect;
    let id = &s.shape.id;
    let help = format!(
        "to put the text inside '{id}', move it into its text block, in box-local coordinates: `box {id} … {{ text {:?} at ({},{}) }}`",
        t.content,
        fmt_num(t.at.x - r.x),
        fmt_num(t.at.y - r.y),
    );
    Some((id.clone(), help))
}

/// The help for a line label with no clear position: how long a run it needs.
fn no_clear_position(
    ctx: &Ctx,
    l: &kashika_model::Line,
    label: &kashika_model::Label,
    shape_hit: bool,
) -> String {
    let w = l.style.stroke_width;
    let len = |m: Marker| match m {
        Marker::Open => Marker::Arrow.length(w),
        m => m.length(w),
    };
    let fallback = format!(
        "no clear label position on {}; move the line or the obstacle",
        l.id
    );
    let Some((from, to)) = l.path.iter().find_map(|s| match *s {
        PathSeg::Line { from, to } if on_segment(label.at, from, to) => Some((from, to)),
        _ => None,
    }) else {
        return fallback;
    };
    let horizontal = (to.x - from.x).abs() >= (to.y - from.y).abs();
    let (extent, dim) = if horizontal {
        (label.rect.w, "wide")
    } else {
        (label.rect.h, "tall")
    };
    let ends = len(l.start).max(len(l.end));
    let need = (extent + 2.0 * ends + ctx.scene.clearance).ceil();
    let run = from.dist(to);
    if !label.side.is_on() || run >= need {
        return fallback;
    }
    let mut help = format!(
        "no clear label position on {}: the label is {}px {dim} and its segment {}px long; give it about {}px clear of boxes and arrowheads (widen the gap), shorten it or break it with `\\n`",
        l.id,
        fmt_num(extent.ceil()),
        fmt_num(run.round()),
        fmt_num(need),
    );
    if !shape_hit {
        let side = [
            (kashika_model::LabelSide::Above, "above"),
            (kashika_model::LabelSide::Left, "left"),
        ]
        .into_iter()
        .find(|(s, _)| s.fits(&l.path, label.t).is_ok());
        if let Some((_, side)) = side {
            help.push_str(&format!(
                ", or put it beside the line with `label-side={side}`"
            ));
        }
    }
    help
}

fn on_segment(p: Pt, a: Pt, b: Pt) -> bool {
    let len = a.dist(b);
    len > 0.0 && (p.dist(a) + p.dist(b) - len).abs() <= 0.5
}

fn describe_clear(line: &str, ts: &[f64]) -> String {
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
