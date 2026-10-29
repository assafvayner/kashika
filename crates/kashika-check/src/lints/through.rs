use kashika_diag::{Diagnostic, code, fact};
use kashika_geom::{Pt, TOLERANCE, inside_runs, strictly_inside};

use crate::ctx::{Ctx, arr, kind_name, line_el, shape_el};
use crate::free::with_free;
use crate::sink::Sink;

/// `through`: a line enters a box's interior. For the boxes a line starts and ends at, only the
/// run from an endpoint strictly inside the box out to its outline is exempt: reaching an
/// endpoint on the outline from inside the box is flagged as running through its own endpoint
/// box, and leaving and re-entering as passing through it.
pub fn through(ctx: &Ctx, out: &mut Sink) {
    for (i, l) in ctx.lines.iter().enumerate() {
        let (starts, ends) = (ctx.attached(l.start), ctx.attached(l.end));
        for s in ctx.boxes_near(ctx.line_boxes[i], TOLERANCE) {
            let id = s.shape.id.as_str();
            let mut runs: Vec<(Pt, Pt, usize)> = Vec::new();
            for &(p, q, k) in &l.pieces {
                for (a, b) in inside_runs(p, q, &s.poly) {
                    match runs.last_mut() {
                        Some(last) if last.1.dist(a) <= TOLERANCE => last.1 = b,
                        _ => runs.push((a, b, k)),
                    }
                }
            }
            let (is_start, is_end) = (starts.contains(&id), ends.contains(&id));
            if is_start && strictly_inside(l.start, &s.poly) {
                runs.retain(|r| r.0.dist(l.start) > TOLERANCE);
            }
            if is_end && strictly_inside(l.end, &s.poly) {
                runs.retain(|r| r.1.dist(l.end) > TOLERANCE);
            }
            let at_end = is_end && runs.last().is_some_and(|r| r.1.dist(l.end) <= TOLERANCE);
            let at_start = is_start && runs.first().is_some_and(|r| r.0.dist(l.start) <= TOLERANCE);
            if at_end || at_start {
                let (entry, exit, k) = if at_end {
                    runs[runs.len() - 1]
                } else {
                    runs[0]
                };
                let help = if at_end {
                    format!(
                        "arrive at '{id}' from outside: keep the last corner beyond the side it ends on, so a `left`/`right` end is reached by a horizontal run from outside (`vh`, or an `x …` hop first) and a `top`/`bottom` end by a vertical one (`hv`, `y …`)"
                    )
                } else {
                    format!(
                        "leave '{id}' straight out of the side it starts on: from `left`/`right` run horizontally clear of it first (`hv`, or an `x …` hop), from `top`/`bottom` vertically (`vh`, `y …`)"
                    )
                };
                out.push(
                    Diagnostic::warn(
                        code::THROUGH,
                        l.line.span,
                        format!(
                            "{} runs through the inside of its own endpoint {} '{id}'",
                            l.line.id,
                            kind_name(s.shape)
                        ),
                    )
                    .with_element(line_el(l.line, Some(l.segment(k))))
                    .with_element(shape_el(s.shape))
                    .with_fact(fact::ENTRY, arr(entry))
                    .with_fact(fact::EXIT, arr(exit))
                    .with_help(help),
                );
                continue;
            }
            let (Some(&(entry, _, k)), Some(&(_, exit, _))) = (runs.first(), runs.last()) else {
                continue;
            };
            let mut skip = vec![l.line.id.as_str()];
            skip.extend(starts.iter().chain(&ends).copied());
            let d = Diagnostic::warn(
                code::THROUGH,
                l.line.span,
                format!("{} passes through {} '{id}'", l.line.id, kind_name(s.shape)),
            )
            .with_element(line_el(l.line, Some(l.segment(k))))
            .with_element(shape_el(s.shape))
            .with_fact(fact::ENTRY, arr(entry))
            .with_fact(fact::EXIT, arr(exit));
            out.push(with_free(d, l, k, &skip));
        }
    }
}
