use serde::Serialize;

use crate::{Pt, TOLERANCE};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn origin(self) -> Pt {
        Pt::new(self.x, self.y)
    }

    pub fn center(self) -> Pt {
        Pt::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }

    /// Maps a point in unit coordinates (0..1 across the rect) to this rect.
    pub fn map_unit(self, p: Pt) -> Pt {
        Pt::new(self.x + p.x * self.w, self.y + p.y * self.h)
    }

    /// Maps a rect in unit coordinates to this rect.
    pub fn map_rect(self, u: Rect) -> Rect {
        Rect::new(
            self.x + u.x * self.w,
            self.y + u.y * self.h,
            u.w * self.w,
            u.h * self.h,
        )
    }

    /// The rect with corners `a` and `b`.
    pub fn from_points(a: Pt, b: Pt) -> Rect {
        Rect::new(
            a.x.min(b.x),
            a.y.min(b.y),
            (a.x - b.x).abs(),
            (a.y - b.y).abs(),
        )
    }

    /// The bounding box of `pts`, or the default rect when there are none.
    pub fn bounding(pts: impl IntoIterator<Item = Pt>) -> Rect {
        let mut it = pts.into_iter();
        let Some(first) = it.next() else {
            return Rect::default();
        };
        let (mut lo, mut hi) = (first, first);
        for p in it {
            lo = Pt::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Pt::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        Rect::new(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y)
    }

    /// The smallest rect holding both.
    pub fn union(self, o: Rect) -> Rect {
        let (x0, y0) = (self.x.min(o.x), self.y.min(o.y));
        let (x1, y1) = (self.right().max(o.right()), self.bottom().max(o.bottom()));
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// The overlap of the two, when it is more than `TOLERANCE` in both directions.
    pub fn intersect(self, o: Rect) -> Option<Rect> {
        let (x0, y0) = (self.x.max(o.x), self.y.max(o.y));
        let (x1, y1) = (self.right().min(o.right()), self.bottom().min(o.bottom()));
        (x1 - x0 > TOLERANCE && y1 - y0 > TOLERANCE).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
    }

    /// Whether `inner` lies within this rect, touching allowed.
    pub fn contains_rect(self, inner: Rect) -> bool {
        inner.x >= self.x - TOLERANCE
            && inner.y >= self.y - TOLERANCE
            && inner.right() <= self.right() + TOLERANCE
            && inner.bottom() <= self.bottom() + TOLERANCE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_points_takes_either_corner_order() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(Rect::from_points(Pt::new(4.0, 6.0), Pt::new(1.0, 2.0)), r);
        assert_eq!(Rect::bounding([Pt::new(4.0, 2.0), Pt::new(1.0, 6.0)]), r);
        assert_eq!(Rect::bounding([]), Rect::default());
    }

    #[test]
    fn union_intersect_and_contain() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.union(b), Rect::new(0.0, 0.0, 15.0, 15.0));
        assert_eq!(a.intersect(b), Some(Rect::new(5.0, 5.0, 5.0, 5.0)));
        assert_eq!(a.intersect(Rect::new(10.0, 0.0, 5.0, 5.0)), None);
        assert!(a.contains_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
        assert!(!a.contains_rect(b));
    }
}
