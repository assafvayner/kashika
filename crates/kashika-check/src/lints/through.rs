use kashika_diag::Diagnostic;
use kashika_geom::{inside_runs, Pt, TOLERANCE};

use crate::ctx::{arr, kind_name, line_el, shape_el, Ctx};
use crate::free::with_free;

/// `through`: a line enters a box's interior. The boxes a line starts and ends at are exempt
/// only on its first and last segments, so leaving and re-entering is flagged.
pub fn through(ctx: &Ctx) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (i, l) in ctx.lines.iter().enumerate() {
        let (starts, ends) = (ctx.attached(l.start), ctx.attached(l.end));
        let last = l.last_segment();
        for s in ctx.boxes_near(ctx.line_boxes[i], TOLERANCE) {
            let id = s.shape.id.as_str();
            let mut runs: Vec<(Pt, Pt, usize)> = Vec::new();
            for &(p, q, k) in &l.pieces {
                let seg = l.segment(k);
                if (seg == 0 && starts.contains(&id)) || (seg == last && ends.contains(&id)) {
                    continue;
                }
                runs.extend(
                    inside_runs(p, q, &s.poly)
                        .into_iter()
                        .map(|(a, b)| (a, b, k)),
                );
            }
            let (Some(&(entry, _, k)), Some(&(_, exit, _))) = (runs.first(), runs.last()) else {
                continue;
            };
            let mut skip = vec![l.line.id.as_str()];
            skip.extend(starts.iter().chain(&ends).copied());
            let d = Diagnostic::warn(
                "through",
                l.line.span,
                format!("{} passes through {} '{id}'", l.line.id, kind_name(s.shape)),
            )
            .with_element(line_el(l.line, Some(l.segment(k))))
            .with_element(shape_el(s.shape))
            .with_fact("entry", arr(entry))
            .with_fact("exit", arr(exit));
            out.push(with_free(ctx, d, l, k, &skip));
        }
    }
    out
}
