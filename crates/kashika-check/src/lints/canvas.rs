use kashika_diag::{Diagnostic, Element, Span};
use kashika_geom::{Rect, fmt_num};
use kashika_model::Shape;

use crate::ctx::{Ctx, describe_overflow, line_bbox, overflow, overflow_fact};
use crate::element::{junction_el, line_el, shape_el, text_el, text_id};
use crate::sink::Sink;
use crate::text_box::{below_box, label_box, text_box};

/// `out-of-canvas`: an element extends beyond the diagram size. Strokes and markers are ignored.
pub fn out_of_canvas(ctx: &Ctx, out: &mut Sink) {
    let (w, h) = ctx.scene.size;
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let canvas = Rect::new(0.0, 0.0, w, h);
    let fitted = ctx.scene.fitted;
    let mut check = |what: String, el: Element, span: Span, extent: Rect| {
        let over = overflow(canvas, extent);
        if over.is_empty() {
            return;
        }
        let mut d = Diagnostic::warn(
            "out-of-canvas",
            span,
            format!(
                "{what} extends beyond the {}x{} canvas ({})",
                fmt_num(w),
                fmt_num(h),
                describe_overflow(&over)
            ),
        )
        .with_element(el)
        .with_fact("overflow", overflow_fact(&over));
        let shifts: Vec<String> = over
            .iter()
            .filter_map(|&(side, by)| match side {
                "left" => Some(format!("right by {}px", fmt_num(by))),
                "top" => Some(format!("down by {}px", fmt_num(by))),
                _ => None,
            })
            .collect();
        if !shifts.is_empty() {
            let fitted = if fitted.0 || fitted.1 {
                "; a fitted canvas grows only right and down"
            } else {
                ""
            };
            d = d.with_help(format!(
                "move it {} (or everything, to keep the layout): the canvas starts at (0,0){fitted}",
                shifts.join(" and ")
            ));
        }
        if shifts.len() < over.len() {
            d = d.with_help(format!(
                "move it inside (0,0)-({},{}) or enlarge `size` on the `diagram` line",
                fmt_num(w),
                fmt_num(h)
            ));
        }
        out.push(d);
    };
    let escapes = |s: &Shape| !overflow(canvas, s.rect).is_empty();
    let inside_escaping_group = |s: &Shape| {
        let mut parent = s.parent.as_deref();
        while let Some(id) = parent {
            let Some(p) = ctx.scene.shapes.iter().find(|x| x.id == id) else {
                return false;
            };
            if escapes(p) {
                return true;
            }
            parent = p.parent.as_deref();
        }
        false
    };
    for s in &ctx.scene.shapes {
        if inside_escaping_group(s) {
            continue;
        }
        check(
            format!("{} '{}'", s.kind.name(), s.id),
            shape_el(s),
            s.span,
            s.rect,
        );
        if let Some(b) = below_box(s) {
            check(
                format!("the label under {} '{}'", s.kind.name(), s.id),
                shape_el(s),
                s.span,
                b,
            );
        }
    }
    for j in &ctx.scene.junctions {
        check(
            format!("junction '{}'", j.id),
            junction_el(j),
            j.span,
            Rect::new(j.at.x, j.at.y, 0.0, 0.0),
        );
    }
    for t in &ctx.scene.texts {
        check(
            format!("text '{}'", text_id(t)),
            text_el(t),
            t.span,
            text_box(t),
        );
    }
    for l in &ctx.lines {
        check(
            format!("line {}", l.line.id),
            line_el(l.line, None),
            l.line.span,
            line_bbox(l),
        );
        for lab in &l.line.labels {
            check(
                format!("label \"{}\" of {}", lab.text, l.line.id),
                line_el(l.line, None),
                lab.span,
                label_box(lab),
            );
        }
    }
}
