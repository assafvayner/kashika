use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, TOLERANCE, boundary_nearest, fmt_num, round2};
use kashika_model::Shape;

use crate::ctx::{Ctx, ShapeGeom, fmt_pt, line_el, outline_anchor};
use crate::sink::Sink;

/// The largest angle off horizontal or vertical that still reads as a slip: 5°.
const MAX_SLANT_DEG: f64 = 5.0;

/// How far a straight piece from `from` to `to` is off the nearest axis, when it is off by
/// more than TOLERANCE but within 5°: (offset in px, angle in degrees, horizontal).
pub fn slant(from: Pt, to: Pt) -> Option<(f64, f64, bool)> {
    let (dx, dy) = ((to.x - from.x).abs(), (to.y - from.y).abs());
    let (major, minor) = (dx.max(dy), dx.min(dy));
    (minor > TOLERANCE && minor <= major * MAX_SLANT_DEG.to_radians().tan())
        .then(|| (minor, minor.atan2(major).to_degrees(), dx >= dy))
}

/// `near-axis`: a straight segment on a line without `ortho` is within 5° of horizontal or
/// vertical but not on it, so it renders as a visible slant.
pub fn near_axis(ctx: &Ctx, out: &mut Sink) {
    for l in ctx.lines.iter().filter(|l| !l.line.ortho) {
        for (i, s) in l.line.path.iter().enumerate() {
            let &PathSeg::Line { from, to } = s else {
                continue;
            };
            let Some((off, deg, horizontal)) = slant(from, to) else {
                continue;
            };
            let seg = l.segment(i);
            let span = l.line.segments.get(seg).map_or(l.line.span, |s| s.span);
            let (axis, coord) = if horizontal {
                ("y", "horizontal")
            } else {
                ("x", "vertical")
            };
            let mut d = Diagnostic::warn(
                "near-axis",
                span,
                format!(
                    "segment {} of {} is {}px ({}°) off {coord}, from {} to {}",
                    seg + 1,
                    l.line.id,
                    fmt_num(off),
                    fmt_num(deg),
                    fmt_pt(from),
                    fmt_pt(to)
                ),
            )
            .with_element(line_el(l.line, Some(seg)))
            .with_fact("offset", round2(off))
            .with_fact("degrees", round2(deg));
            let level = |keep: Pt, mv: Pt| {
                if horizontal {
                    Pt::new(mv.x, keep.y)
                } else {
                    Pt::new(keep.x, mv.y)
                }
            };
            let value = |p: Pt| fmt_num(if horizontal { p.y } else { p.x });
            let mut fixes = Vec::new();
            if i + 1 == l.line.path.len() {
                if let Some(a) = anchor_at(ctx, to, level(from, to)) {
                    fixes.push(format!("end at `{a}` to keep {axis}={}", value(from)));
                }
            }
            if i == 0 {
                if let Some(a) = anchor_at(ctx, from, level(to, from)) {
                    fixes.push(format!("start at `{a}` to keep {axis}={}", value(to)));
                }
            }
            let or = if fixes.is_empty() { "" } else { "or " };
            fixes.push(format!(
                "{or}move one end {}px in {axis} so both ends share it, or use `hv`/`vh` (or an `x`/`y` target) for a deliberate elbow",
                fmt_num(off)
            ));
            for f in fixes {
                d = d.with_help(f);
            }
            out.push(d);
        }
    }
}

/// An anchor for `q` on the box that `at` attaches to, when `q` lies on its outline: `id.side@t`
/// when that lands within 0.25px of `q`, otherwise `id.side+(dx,dy)`.
pub(crate) fn anchor_at(ctx: &Ctx, at: Pt, q: Pt) -> Option<String> {
    let id = ctx.attached(at).into_iter().next()?;
    let geom: &ShapeGeom = ctx.boxes().find(|s| s.shape.id == id)?;
    if boundary_nearest(q, &geom.poly).0 > TOLERANCE {
        return None;
    }
    Some(anchor_text(geom.shape, q))
}

fn anchor_text(s: &Shape, q: Pt) -> String {
    let (side, t) = outline_anchor(s, q);
    let t = round2(t);
    if s.anchor(side, Some(t)).is_some_and(|p| p.dist(q) <= 0.25) {
        return if t == 0.5 {
            format!("{}.{side}", s.id)
        } else {
            format!("{}.{side}@{}", s.id, fmt_num(t))
        };
    }
    let c = s.anchor(side, None).unwrap_or(q);
    format!(
        "{}.{side}+({},{})",
        s.id,
        fmt_num(q.x - c.x),
        fmt_num(q.y - c.y)
    )
}
