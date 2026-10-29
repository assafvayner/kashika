use kashika_diag::Span;
use kashika_syntax::ast::{self, Coord, Point, SegKind, Target};

use super::Resolver;
use crate::scene::{EdgeKind, Literal, PlacementEdge, SegmentKind};

impl Resolver<'_> {
    /// Records the placement edges of a box, group or junction: its frame (parent group)
    /// and every id its `at` point references.
    pub(super) fn record_edges(&mut self, idx: usize, at: &Point) {
        let Some(from) = self.entries[idx].item.id().map(|i| i.name.clone()) else {
            return;
        };
        if let Some(parent) = self.entries[idx]
            .parent
            .and_then(|p| self.entries[p].item.id())
        {
            self.edges.push(PlacementEdge {
                from: from.clone(),
                to: parent.name.clone(),
                kind: EdgeKind::Frame,
            });
        }
        for r in point_refs(at) {
            let e = PlacementEdge {
                from: from.clone(),
                to: r.id.name.clone(),
                kind: EdgeKind::At,
            };
            if !self.edges.contains(&e) {
                self.edges.push(e);
            }
        }
    }

    pub(super) fn literal(&mut self, owner: &str, what: &'static str, value: f64, span: Span) {
        self.literals.push(Literal {
            owner: owner.to_string(),
            what,
            value,
            span,
        });
    }

    pub(super) fn record_point(&mut self, owner: &str, p: &Point) {
        match p {
            Point::Lit { x, y, span } => {
                if let Coord::Num(v) = x {
                    self.literal(owner, "x", *v, *span);
                }
                if let Coord::Num(v) = y {
                    self.literal(owner, "y", *v, *span);
                }
            }
            Point::Ref {
                offset: Some(o),
                span,
                ..
            } => {
                let (dx, dy) = o.delta();
                self.literal(owner, "dx", dx, *span);
                self.literal(owner, "dy", dy, *span);
            }
            Point::Ref { offset: None, .. } => {}
            Point::Group { inner, .. } => self.record_point(owner, inner),
            Point::Shifted {
                base,
                offsets,
                span,
            } => {
                self.record_point(owner, base);
                for o in offsets {
                    let (dx, dy) = o.delta();
                    self.literal(owner, "dx", dx, *span);
                    self.literal(owner, "dy", dy, *span);
                }
            }
        }
    }

    /// Records a box or group's `at` literals and the axes of its size that are numbers.
    pub(super) fn record_node(&mut self, n: &ast::Node) {
        self.record_point(&n.id.name, &n.at);
        let size = n.size.unwrap_or(ast::Size { w: None, h: None });
        if let Some(w) = size.w {
            self.literal(&n.id.name, "width", w, n.span);
        }
        if let Some(h) = size.h {
            self.literal(&n.id.name, "height", h, n.span);
        }
    }

    pub(super) fn record_line(&mut self, owner: &str, l: &ast::Line) {
        self.record_point(owner, &l.start);
        for seg in &l.segs {
            self.record_segment(owner, seg);
        }
    }

    fn record_segment(&mut self, owner: &str, seg: &ast::Seg) {
        match &seg.kind {
            SegKind::To(p)
            | SegKind::Hv(p)
            | SegKind::Vh(p)
            | SegKind::Vhv { to: p, .. }
            | SegKind::Hvh { to: p, .. }
            | SegKind::Curve { to: p, .. } => self.record_point(owner, p),
            SegKind::H(d) => self.literal(owner, "h", *d, seg.span),
            SegKind::V(d) => self.literal(owner, "v", *d, seg.span),
            SegKind::X(t) => self.record_target(owner, "x", t, seg.span),
            SegKind::Y(t) => self.record_target(owner, "y", t, seg.span),
            SegKind::CurveVia { c1, c2, to } => {
                for p in [c1, c2, to] {
                    self.record_point(owner, p);
                }
            }
        }
    }

    fn record_target(&mut self, owner: &str, what: &'static str, t: &Target, span: Span) {
        match t {
            Target::Num(n) => self.literal(owner, what, *n, span),
            Target::Point(p) => self.record_point(owner, p),
        }
    }
}

/// The ids a point references: its base, or the refs in its `x`/`y` coordinates.
pub(super) fn point_refs(p: &Point) -> Vec<&ast::Ref> {
    match p.base() {
        Point::Ref { r, .. } => vec![r],
        Point::Lit { x, y, .. } => [x, y]
            .into_iter()
            .filter_map(|c| match c {
                Coord::Axis { r, .. } => Some(r),
                Coord::Num(_) => None,
            })
            .collect(),
        Point::Group { .. } | Point::Shifted { .. } => Vec::new(),
    }
}

pub(super) fn segment_kind(k: &SegKind) -> SegmentKind {
    match k {
        SegKind::To(_) => SegmentKind::To,
        SegKind::H(_) => SegmentKind::H,
        SegKind::V(_) => SegmentKind::V,
        SegKind::X(_) => SegmentKind::X,
        SegKind::Y(_) => SegmentKind::Y,
        SegKind::Hv(_) => SegmentKind::Hv,
        SegKind::Vh(_) => SegmentKind::Vh,
        SegKind::Vhv { .. } => SegmentKind::Vhv,
        SegKind::Hvh { .. } => SegmentKind::Hvh,
        SegKind::Curve { .. } => SegmentKind::Curve,
        SegKind::CurveVia { .. } => SegmentKind::CurveVia,
    }
}

pub(super) fn is_literal(p: &Point) -> bool {
    !matches!(p.base(), Point::Ref { .. })
}

pub(super) fn end_is_literal(segs: &[ast::Seg]) -> bool {
    match segs.last().map(|s| &s.kind) {
        Some(
            SegKind::To(p)
            | SegKind::Hv(p)
            | SegKind::Vh(p)
            | SegKind::Vhv { to: p, .. }
            | SegKind::Hvh { to: p, .. }
            | SegKind::Curve { to: p, .. }
            | SegKind::CurveVia { to: p, .. },
        ) => is_literal(p),
        Some(SegKind::X(Target::Point(_)) | SegKind::Y(Target::Point(_))) => false,
        _ => true,
    }
}
