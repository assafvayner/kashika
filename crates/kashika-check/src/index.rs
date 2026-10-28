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
}
