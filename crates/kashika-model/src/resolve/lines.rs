use kashika_diag::Diagnostic;
use kashika_geom::{PathSeg, Pt, TOLERANCE, point_at, round_corners_tagged};
use kashika_syntax::ast::{self, Axis, SegKind, Side};

use crate::scene::*;
use crate::textlayout;

use super::Resolver;
use super::record;

impl<'a> Resolver<'a> {
    pub(super) fn build_line(
        &mut self,
        idx: usize,
        l: &'a ast::Line,
        parent: Option<String>,
    ) -> Option<Line> {
        let origin = self.frame_origin(idx)?;
        let mut cur = self.eval_point(&l.start, origin)?;
        let mut raw: Vec<(PathSeg, usize)> = Vec::new();
        let mut segments: Vec<Segment> = Vec::new();
        let mut failed = false;
        for seg in &l.segs {
            let from = cur;
            let Some(segs) = self.expand(seg, cur, origin) else {
                failed = true;
                break;
            };
            let mut pieces = Vec::new();
            for s in segs {
                cur = s.to();
                if matches!(s, PathSeg::Line { .. }) && s.from().dist(s.to()) <= 1e-9 {
                    continue;
                }
                raw.push((s, segments.len()));
                pieces.push(s);
            }
            segments.push(Segment {
                kind: record::segment_kind(&seg.kind),
                span: seg.span,
                from,
                to: cur,
                pieces,
            });
        }
        if failed {
            return None;
        }
        if raw.is_empty() {
            self.diags.push(Diagnostic::error(
                "zero-length",
                l.span,
                "line has zero length",
            ));
            return None;
        }
        let p = self.lines.remove(&idx)?;
        if p.ortho {
            for (s, i) in &raw {
                if !s.is_axis_aligned() {
                    self.diags.push(
                        Diagnostic::error(
                            "not-ortho",
                            segments[*i].span,
                            "segment is not horizontal or vertical on a line marked `ortho`",
                        )
                        .with_help("use `hv`/`vh` elbows or `x`/`y` targets"),
                    );
                }
            }
        }
        let rounded = if p.radius > 0.0 {
            round_corners_tagged(&raw, p.radius)
        } else {
            raw
        };
        let (path, path_seg): (Vec<PathSeg>, Vec<usize>) = rounded.into_iter().unzip();
        let mut labels = p.labels;
        let bold = p.style.font_weight == crate::style::FontWeight::Bold;
        labels.retain_mut(|lab| {
            let Some(block) = textlayout::measure_at(
                &lab.text,
                p.style.font_size,
                bold,
                lab.span,
                &mut self.diags,
            ) else {
                return false;
            };
            lab.at = point_at(&path, lab.t);
            (lab.lines, lab.rect) =
                block.place_at(lab.at.x, lab.at.y, kashika_text::HAlign::Center);
            true
        });
        let (id, named) = match &l.id {
            Some(i) => (i.name.clone(), true),
            None => (format!("line@L{}", l.span.line), false),
        };
        self.record_line(&id, l);
        Some(Line {
            id,
            named,
            path,
            segments,
            path_seg,
            start_literal: record::is_literal(&l.start),
            end_literal: record::end_is_literal(&l.segs),
            start: p.start,
            end: p.end,
            style: p.style,
            labels,
            ortho: p.ortho,
            allow: p.allow,
            reason: p.reason,
            parent,
            origin,
            span: l.span,
        })
    }

    pub(super) fn expand(
        &mut self,
        seg: &'a ast::Seg,
        cur: Pt,
        origin: Pt,
    ) -> Option<Vec<PathSeg>> {
        let line = |a: Pt, b: Pt| PathSeg::Line { from: a, to: b };
        Some(match &seg.kind {
            SegKind::To(p) => vec![line(cur, self.eval_point(p, origin)?)],
            SegKind::H(dx) => {
                let x = cur.x + dx;
                self.finite(&[x], seg.span).then_some(())?;
                vec![line(cur, Pt::new(x, cur.y))]
            }
            SegKind::V(dy) => {
                let y = cur.y + dy;
                self.finite(&[y], seg.span).then_some(())?;
                vec![line(cur, Pt::new(cur.x, y))]
            }
            SegKind::X(t) => {
                let x = self.eval_target(t, origin, Axis::X)?;
                self.finite(&[x], seg.span).then_some(())?;
                vec![line(cur, Pt::new(x, cur.y))]
            }
            SegKind::Y(t) => {
                let y = self.eval_target(t, origin, Axis::Y)?;
                self.finite(&[y], seg.span).then_some(())?;
                vec![line(cur, Pt::new(cur.x, y))]
            }
            SegKind::Hv(p) => {
                let p = self.eval_point(p, origin)?;
                let corner = Pt::new(p.x, cur.y);
                vec![line(cur, corner), line(corner, p)]
            }
            SegKind::Vh(p) => {
                let p = self.eval_point(p, origin)?;
                let corner = Pt::new(cur.x, p.y);
                vec![line(cur, corner), line(corner, p)]
            }
            SegKind::Curve { side, amount, to } => {
                let to = self.eval_point(to, origin)?;
                self.finite(&[*amount], seg.span).then_some(())?;
                if *amount < 0.0 {
                    self.diags.push(Diagnostic::error(
                        "degenerate-curve",
                        seg.span,
                        "curve amount must not be negative; use `left`/`right` to choose the side",
                    ));
                    return None;
                }
                if cur.dist(to) <= TOLERANCE {
                    self.diags.push(
                        Diagnostic::error(
                            "degenerate-curve",
                            seg.span,
                            "curve starts and ends at the same point",
                        )
                        .with_help("use `curve via (c1) (c2) <point>` for loops"),
                    );
                    return None;
                }
                let d = (to - cur).norm();
                // Screen-left of the travel direction in y-down coordinates.
                let normal = match side {
                    Side::Left => Pt::new(d.y, -d.x),
                    Side::Right => Pt::new(-d.y, d.x),
                };
                let ctrl = cur.lerp(to, 0.5) + normal * (2.0 * amount);
                vec![PathSeg::Quad {
                    from: cur,
                    ctrl,
                    to,
                }]
            }
            SegKind::CurveVia { c1, c2, to } => {
                let c1 = self.eval_point(c1, origin)?;
                let c2 = self.eval_point(c2, origin)?;
                let to = self.eval_point(to, origin)?;
                if [c1, c2, to].iter().all(|p| p.dist(cur) <= TOLERANCE) {
                    self.diags.push(Diagnostic::error(
                        "degenerate-curve",
                        seg.span,
                        "curve has zero size",
                    ));
                    return None;
                }
                vec![PathSeg::Cubic {
                    from: cur,
                    c1,
                    c2,
                    to,
                }]
            }
        })
    }
}
