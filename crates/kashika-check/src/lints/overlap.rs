use kashika_diag::Diagnostic;
use kashika_geom::{TOLERANCE, polygons_overlap};
use kashika_model::ShapeKind;

use crate::ctx::Ctx;
use crate::element::{bbox, kind_name, shape_el};
use crate::sink::Sink;

/// `overlap`: two shapes have intersecting interiors and neither is a group that contains the other.
/// Shapes nested in the source (ancestor and descendant) are skipped; `spill` covers those.
pub fn overlap(ctx: &Ctx, out: &mut Sink) {
    for (i, a) in ctx.shapes.iter().enumerate() {
        for j in ctx.shapes_near(ctx.shape_boxes[i], TOLERANCE) {
            if j <= i {
                continue;
            }
            let b = &ctx.shapes[j];
            let (sa, sb) = (a.shape, b.shape);
            if !sa.layer.is_normal()
                || !sb.layer.is_normal()
                || ctx.is_ancestor(&sa.id, &sb.id)
                || ctx.is_ancestor(&sb.id, &sa.id)
            {
                continue;
            }
            let group_holds = |outer: &kashika_model::Shape, inner: &kashika_model::Shape| {
                outer.kind == ShapeKind::Group && outer.rect.contains_rect(inner.rect)
            };
            if group_holds(sa, sb) || group_holds(sb, sa) || !polygons_overlap(&a.poly, &b.poly) {
                continue;
            }
            let mut d = Diagnostic::warn(
                "overlap",
                sb.span,
                format!(
                    "{} '{}' overlaps {} '{}'",
                    kind_name(sb),
                    sb.id,
                    kind_name(sa),
                    sa.id
                ),
            )
            .with_element(shape_el(sb))
            .with_element(shape_el(sa));
            if let Some(r) = sa.rect.intersect(sb.rect) {
                d = d.with_fact("intersection", bbox(r));
            }
            if sa.rect.contains_rect(sb.rect) || sb.rect.contains_rect(sa.rect) {
                d = d.with_help("only a group may contain another shape; make the outer one a `group` or move the inner one out");
            }
            out.push(d);
        }
    }
}
