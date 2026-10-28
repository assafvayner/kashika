use kashika_geom::Rect;

/// A uniform grid over bounding boxes, for finding the few that lie near a query box.
pub struct Grid {
    x0: f64,
    y0: f64,
    cell: f64,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<usize>>,
    /// Boxes with non-finite coordinates: candidates for every query.
    wild: Vec<usize>,
    len: usize,
}

impl Grid {
    pub fn new(rects: &[Rect]) -> Self {
        let finite = |r: &Rect| [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite());
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for r in rects.iter().filter(|r| finite(r)) {
            x0 = x0.min(r.x.min(r.right()));
            y0 = y0.min(r.y.min(r.bottom()));
            x1 = x1.max(r.x.max(r.right()));
            y1 = y1.max(r.y.max(r.bottom()));
        }
        let n = rects.len().max(1) as f64;
        let (w, h) = ((x1 - x0).max(0.0), (y1 - y0).max(0.0));
        let cell = (w * h / n).sqrt().max(w.max(h) / n).max(1.0);
        let (cols, rows) = if x1 < x0 {
            (1, 1)
        } else {
            ((w / cell) as usize + 1, (h / cell) as usize + 1)
        };
        let mut grid = Grid {
            x0,
            y0,
            cell,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
            wild: Vec::new(),
            len: rects.len(),
        };
        for (i, r) in rects.iter().enumerate() {
            if !finite(r) {
                grid.wild.push(i);
                continue;
            }
            let (c0, c1, r0, r1) = grid.span(*r, 0.0);
            for row in r0..=r1 {
                for col in c0..=c1 {
                    grid.cells[row * cols + col].push(i);
                }
            }
        }
        grid
    }

    /// Column and row ranges of the cells that `r`, grown by `margin`, touches.
    fn span(&self, r: Rect, margin: f64) -> (usize, usize, usize, usize) {
        let col = |x: f64| (((x - self.x0) / self.cell).max(0.0) as usize).min(self.cols - 1);
        let row = |y: f64| (((y - self.y0) / self.cell).max(0.0) as usize).min(self.rows - 1);
        (
            col(r.x.min(r.right()) - margin),
            col(r.x.max(r.right()) + margin),
            row(r.y.min(r.bottom()) - margin),
            row(r.y.max(r.bottom()) + margin),
        )
    }

    /// Indices, ascending, of every box that may lie within `margin` of `r`: a superset of
    /// the boxes that do.
    pub fn near(&self, r: Rect, margin: f64) -> Vec<usize> {
        if ![r.x, r.y, r.w, r.h, margin].iter().all(|v| v.is_finite()) {
            return (0..self.len).collect();
        }
        let (c0, c1, r0, r1) = self.span(r, margin.max(0.0));
        let mut out = self.wild.clone();
        for row in r0..=r1 {
            for col in c0..=c1 {
                out.extend_from_slice(&self.cells[row * self.cols + col]);
            }
        }
        if out.len() < self.len / 16 {
            out.sort_unstable();
            out.dedup();
            return out;
        }
        let mut hit = vec![false; self.len];
        for i in out {
            hit[i] = true;
        }
        (0..self.len).filter(|&i| hit[i]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nearby_boxes_in_order() {
        let rects: Vec<Rect> = (0..100)
            .map(|i| {
                Rect::new(
                    f64::from(i % 10) * 50.0,
                    f64::from(i / 10) * 50.0,
                    20.0,
                    20.0,
                )
            })
            .collect();
        let grid = Grid::new(&rects);
        let near = grid.near(Rect::new(105.0, 105.0, 0.0, 0.0), 10.0);
        assert!(near.contains(&22));
        assert!(near.windows(2).all(|w| w[0] < w[1]));
        assert!(!near.contains(&99));
        assert_eq!(
            grid.near(Rect::new(f64::NAN, 0.0, 0.0, 0.0), 0.0).len(),
            100
        );
    }

    /// A small deterministic generator, so the property test needs no dependency.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self, lo: f64, hi: f64) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            lo + (hi - lo) * ((self.0 >> 11) as f64 / (1u64 << 53) as f64)
        }
    }

    fn near_brute(rects: &[Rect], q: Rect, margin: f64) -> Vec<usize> {
        let finite = |r: &Rect| [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite());
        let (qx0, qx1) = (q.x.min(q.right()), q.x.max(q.right()));
        let (qy0, qy1) = (q.y.min(q.bottom()), q.y.max(q.bottom()));
        (0..rects.len())
            .filter(|&i| {
                let r = rects[i];
                !finite(&r)
                    || (r.x.min(r.right()) - margin <= qx1
                        && r.x.max(r.right()) + margin >= qx0
                        && r.y.min(r.bottom()) - margin <= qy1
                        && r.y.max(r.bottom()) + margin >= qy0)
            })
            .collect()
    }

    #[test]
    fn grid_finds_everything_brute_force_finds() {
        let mut rng = Lcg(7);
        for round in 0..50 {
            let n = round * 7 % 120;
            let mut rects: Vec<Rect> = (0..n)
                .map(|_| {
                    Rect::new(
                        rng.next(-500.0, 1500.0),
                        rng.next(-500.0, 1500.0),
                        rng.next(-50.0, 300.0),
                        rng.next(-50.0, 300.0),
                    )
                })
                .collect();
            if round % 5 == 0 && n > 0 {
                rects[n / 2] = Rect::new(f64::NAN, 0.0, 10.0, 10.0);
            }
            let grid = Grid::new(&rects);
            for _ in 0..40 {
                let q = Rect::new(
                    rng.next(-700.0, 1700.0),
                    rng.next(-700.0, 1700.0),
                    rng.next(-100.0, 400.0),
                    rng.next(-100.0, 400.0),
                );
                let margin = rng.next(0.0, 30.0);
                let got = grid.near(q, margin);
                assert!(got.windows(2).all(|w| w[0] < w[1]), "{got:?}");
                assert!(got.iter().all(|&i| i < rects.len()));
                for i in near_brute(&rects, q, margin) {
                    assert!(got.contains(&i), "round {round}: missed {i} for {q:?}");
                }
            }
        }
    }

    #[test]
    fn an_empty_grid_finds_nothing() {
        let grid = Grid::new(&[]);
        assert!(grid.near(Rect::new(0.0, 0.0, 10.0, 10.0), 5.0).is_empty());
        assert!(
            grid.near(Rect::new(f64::NAN, 0.0, 0.0, 0.0), 0.0)
                .is_empty()
        );
    }

    #[test]
    fn non_finite_rects_are_found_by_every_query() {
        let rects = [
            Rect::new(f64::NAN, f64::NAN, 0.0, 0.0),
            Rect::new(0.0, 0.0, f64::INFINITY, 1.0),
        ];
        let grid = Grid::new(&rects);
        assert_eq!(grid.near(Rect::new(5.0, 5.0, 1.0, 1.0), 0.0), vec![0, 1]);
    }
}
