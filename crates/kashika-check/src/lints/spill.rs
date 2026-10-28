use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{Pt, Rect, inside_runs, rect_polygon};

use crate::ctx::{
    Ctx, bbox, describe_overflow, fmt_num, intersection, junction_el, kind_name, line_bbox,
    line_el, overflow, overflow_fact, shape_el, text_el, text_id,
};
use crate::text_box::{below_box, text_box, title_band};

struct Child {
    el: Element,
    span: Span,
    what: String,
    extent: Rect,
    /// Straight pieces for lines; empty for everything else.
    pieces: Vec<(Pt, Pt)>,
    /// A zero-size junction: it can spill but never intrudes on the title band.
    point: bool,
}

/// `spill`: a child extends outside its group or into the group's title band. Touching is fine.
pub fn spill(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for g in ctx.groups() {
        let gid = g.shape.id.as_str();
        let band = title_band(g.shape);
        for c in children(ctx, gid) {
            let over = overflow(g.shape.rect, c.extent);
            if !over.is_empty() {
                out.push(
                    Diagnostic::warn(
                        "spill",
                        c.span,
                        format!(
                            "{} extends outside group '{gid}' ({})",
                            c.what,
                            describe_overflow(&over)
                        ),
                    )
                    .with_element(c.el.clone())
                    .with_element(shape_el(g.shape))
                    .with_fact("overflow", overflow_fact(&over))
                    .with_help(format!("enlarge '{gid}' or move the child inside it")),
                );
            }
            let Some(b) = band else { continue };
            let intrudes = if c.point {
                false
            } else if c.pieces.is_empty() {
                intersection(b, c.extent).is_some()
            } else {
                let poly = rect_polygon(b);
                c.pieces
                    .iter()
                    .any(|&(p, q)| !inside_runs(p, q, &poly).is_empty())
            };
            if intrudes {
                out.push(
                    Diagnostic::warn(
                        "spill",
                        c.span,
                        format!("{} intrudes on the title band of group '{gid}'", c.what),
                    )
                    .with_element(c.el)
                    .with_element(shape_el(g.shape))
                    .with_fact("band", bbox(b))
                    .with_help(format!("keep children below local y={}", fmt_num(b.h))),
                );
            }
        }
    }
    out
}

fn children(ctx: &Ctx, gid: &str) -> Vec<Child> {
    let mut out = Vec::new();
    let mine = |p: &Option<String>| p.as_deref() == Some(gid);
    for s in ctx.scene.shapes.iter().filter(|s| mine(&s.parent)) {
        out.push(Child {
            el: shape_el(s),
            span: s.span,
            what: format!("{} '{}'", kind_name(s), s.id),
            extent: s.rect,
            pieces: Vec::new(),
            point: false,
        });
        if let Some(b) = below_box(s) {
            out.push(Child {
                el: shape_el(s),
                span: s.span,
                what: format!("the label under {} '{}'", kind_name(s), s.id),
                extent: b,
                pieces: Vec::new(),
                point: false,
            });
        }
    }
    for j in ctx.scene.junctions.iter().filter(|j| mine(&j.parent)) {
        out.push(Child {
            el: junction_el(j),
            span: j.span,
            what: format!("junction '{}'", j.id),
            extent: Rect::new(j.at.x, j.at.y, 0.0, 0.0),
            pieces: Vec::new(),
            point: true,
        });
    }
    for t in ctx.scene.texts.iter().filter(|t| mine(&t.parent)) {
        out.push(Child {
            el: text_el(t),
            span: t.span,
            what: format!("text '{}'", text_id(t)),
            extent: text_box(t),
            pieces: Vec::new(),
            point: false,
        });
    }
    for l in ctx.lines.iter().filter(|l| mine(&l.line.parent)) {
        out.push(Child {
            el: line_el(l.line, None),
            span: l.line.span,
            what: format!("line {}", l.line.id),
            extent: line_bbox(l),
            pieces: l.pieces.iter().map(|&(p, q, _)| (p, q)).collect(),
            point: false,
        });
    }
    out
}
