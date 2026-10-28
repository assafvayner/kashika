use kashika_diag::Diagnostic;
use kashika_geom::{boundary_nearest, inside_runs, point_in_polygon, Pt, TOLERANCE};

use crate::ctx::{arr, line_el, shape_el, Ctx};

/// `group-transit`: a line passes through a group's interior while neither endpoint is inside it.
pub fn group_transit(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for l in &ctx.lines {
        for g in ctx.groups() {
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
            out.push(
                Diagnostic::warn(
                    "group-transit",
                    l.line.span,
                    format!(
                        "{} passes through group '{}' without connecting to anything inside it",
                        l.line.id, g.shape.id
                    ),
                )
                .with_element(line_el(l.line, Some(l.segment(k))))
                .with_element(shape_el(g.shape))
                .with_fact("entry", arr(entry))
                .with_help("route around the group, for example along its outside margin"),
            );
        }
    }
    out
}
