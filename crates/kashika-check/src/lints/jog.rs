use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, TOLERANCE, dot, fmt_num, round2};

use crate::ctx::{Ctx, fmt_pt, line_el};
use crate::lints::axis::anchor_at;
use crate::sink::Sink;

/// A maximal horizontal or vertical stretch of a line's unrounded path.
struct Run {
    from: Pt,
    to: Pt,
    horizontal: bool,
    /// Index into `line.segments` of the segment that drew the start of the run.
    seg: usize,
}

/// `jog`: a short horizontal or vertical step, shorter than `clearance`, between two
/// parallel runs that go the same way, so the line shows a small Z or S kink.
pub fn jog(ctx: &Ctx, out: &mut Sink) {
    let limit = ctx.scene.clearance;
    for l in &ctx.lines {
        let runs = runs(l.line);
        let last = runs.len().saturating_sub(1);
        for (i, w) in runs.windows(3).enumerate() {
            let [Some(a), Some(b), Some(c)] = w else {
                continue;
            };
            let step = b.from.dist(b.to);
            if a.horizontal != c.horizontal
                || b.horizontal == a.horizontal
                || step <= TOLERANCE
                || step >= limit
                || dot(a.to - a.from, c.to - c.from) <= 0.0
            {
                continue;
            }
            let (axis, runs_dir) = if a.horizontal {
                ("y", "horizontal")
            } else {
                ("x", "vertical")
            };
            let value = |p: Pt| if a.horizontal { p.y } else { p.x };
            let with = |p: Pt, v: f64| {
                if a.horizontal {
                    Pt::new(p.x, v)
                } else {
                    Pt::new(v, p.y)
                }
            };
            let span = l.line.segments.get(b.seg).map_or(l.line.span, |s| s.span);
            let mut d = Diagnostic::warn(
                "jog",
                span,
                format!(
                    "segment {} of {} jogs {}px in {axis} between two {runs_dir} runs, from {} to {}",
                    b.seg + 1,
                    l.line.id,
                    fmt_num(step),
                    fmt_pt(b.from),
                    fmt_pt(b.to)
                ),
            )
            .with_element(line_el(l.line, Some(b.seg)))
            .with_fact("step", round2(step))
            .with_fact("clearance", round2(limit));
            let (before, after) = (value(a.to), value(c.from));
            let mut fixes = Vec::new();
            if i + 2 == last {
                if let Some(anc) = anchor_at(ctx, c.to, with(c.to, before)) {
                    fixes.push(format!("end at `{anc}` to keep {axis}={}", fmt_num(before)));
                }
            }
            if i == 0 {
                if let Some(anc) = anchor_at(ctx, a.from, with(a.from, after)) {
                    fixes.push(format!(
                        "start at `{anc}` to keep {axis}={}",
                        fmt_num(after)
                    ));
                }
            }
            let or = if fixes.is_empty() { "" } else { "or " };
            fixes.push(format!(
                "{or}move the run after the step to {axis}={} or the run before it to {axis}={}, so both {runs_dir} runs share {axis}",
                fmt_num(before),
                fmt_num(after),
            ));
            for f in fixes {
                d = d.with_help(f);
            }
            out.push(d);
        }
    }
}

/// The line's path before corner rounding, as horizontal or vertical runs; consecutive
/// pieces on one axis going the same way merge. Curves and diagonals are `None`.
fn runs(line: &kashika_model::Line) -> Vec<Option<Run>> {
    let mut out: Vec<Option<Run>> = Vec::new();
    for (k, s) in line.segments.iter().enumerate() {
        for p in &s.pieces {
            let &PathSeg::Line { from, to } = p else {
                out.push(None);
                continue;
            };
            let (dx, dy) = ((to.x - from.x).abs(), (to.y - from.y).abs());
            let horizontal = if dy <= TOLERANCE && dx > dy {
                true
            } else if dx <= TOLERANCE && dy > dx {
                false
            } else {
                out.push(None);
                continue;
            };
            if let Some(Some(r)) = out.last_mut() {
                if r.horizontal == horizontal && dot(r.to - r.from, to - from) > 0.0 {
                    r.to = to;
                    continue;
                }
            }
            out.push(Some(Run {
                from,
                to,
                horizontal,
                seg: k,
            }));
        }
    }
    out
}
