use kashika_diag::{Diagnostic, code, fact};
use kashika_geom::{
    Pt, Rect, TOLERANCE, boundary_nearest, dist_point_seg, inside_runs, point_in_polygon,
};

use crate::ctx::{Ctx, LineGeom};
use crate::element::{arr, line_el, shape_el};
use crate::sink::Sink;

/// `group-transit`: a line passes through a group's interior while neither endpoint is inside
/// it, and no other line that meets it at a junction has an endpoint inside it either.
pub fn group_transit(ctx: &Ctx, out: &mut Sink) {
    for (i, l) in ctx.lines.iter().enumerate() {
        for g in ctx.groups_near(ctx.line_boxes[i], TOLERANCE) {
            let inside =
                |p: Pt| point_in_polygon(p, &g.poly) || boundary_nearest(p, &g.poly).0 <= TOLERANCE;
            if inside(l.start) || inside(l.end) {
                continue;
            }
            let hit = l
                .pieces
                .iter()
                .find_map(|&(p, q, k)| inside_runs(p, q, &g.poly).first().map(|r| (r.0, k)));
            let Some((entry, k)) = hit else { continue };
            if taps(ctx, i).any(|t| inside(t.start) || inside(t.end)) {
                continue;
            }
            out.push(
                Diagnostic::warn(
                    code::GROUP_TRANSIT,
                    l.line.span,
                    format!(
                        "{} passes through group '{}' without connecting to anything inside it",
                        l.line.id, g.shape.id
                    ),
                )
                .with_element(line_el(l.line, Some(l.segment(k))))
                .with_element(shape_el(g.shape))
                .with_fact(fact::ENTRY, arr(entry))
                .with_help("route around the group, for example along its outside margin"),
            );
        }
    }
}

/// The other lines with an endpoint at a junction that lies on line `i`.
fn taps<'c, 'a>(ctx: &'c Ctx<'a>, i: usize) -> impl Iterator<Item = &'c LineGeom<'a>> + 'c {
    let l = &ctx.lines[i];
    ctx.junctions_near(ctx.line_boxes[i], TOLERANCE)
        .filter(|j| {
            l.pieces
                .iter()
                .any(|&(p, q, _)| dist_point_seg(j.at, p, q) <= TOLERANCE)
        })
        .flat_map(move |j| {
            let at = Rect::new(j.at.x, j.at.y, 0.0, 0.0);
            ctx.lines_near(at, TOLERANCE)
                .into_iter()
                .filter(move |&t| t != i)
                .map(|t| &ctx.lines[t])
                .filter(move |t| t.start.dist(j.at) <= TOLERANCE || t.end.dist(j.at) <= TOLERANCE)
        })
}
