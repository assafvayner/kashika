import { asUsize, fmax, fmin } from "../diag/rust.js";
import { type Rect } from "../geom/index.js";

function finite(r: Rect): boolean {
  return [r.x, r.y, r.w, r.h].every((v) => Number.isFinite(v));
}

/// A uniform grid over bounding boxes, for finding the few that lie near a query box.
export class Grid {
  private constructor(
    private x0: number,
    private y0: number,
    private cell: number,
    private cols: number,
    private rows: number,
    private cells: number[][],
    /// Boxes with non-finite coordinates: candidates for every query.
    private wild: number[],
    private len: number,
  ) {}

  static new(rects: readonly Rect[]): Grid {
    let [x0, y0, x1, y1] = [Number.MAX_VALUE, Number.MAX_VALUE, -Number.MAX_VALUE, -Number.MAX_VALUE];
    for (const r of rects.filter(finite)) {
      x0 = fmin(x0, fmin(r.x, r.right()));
      y0 = fmin(y0, fmin(r.y, r.bottom()));
      x1 = fmax(x1, fmax(r.x, r.right()));
      y1 = fmax(y1, fmax(r.y, r.bottom()));
    }
    const n = Math.max(rects.length, 1);
    const [w, h] = [fmax(x1 - x0, 0.0), fmax(y1 - y0, 0.0)];
    const cell = fmax(fmax(Math.sqrt((w * h) / n), fmax(w, h) / n), 1.0);
    const [cols, rows] = x1 < x0 ? [1, 1] : [asUsize(w / cell) + 1, asUsize(h / cell) + 1];
    const cells: number[][] = [];
    for (let i = 0; i < cols * rows; i++) cells.push([]);
    const grid = new Grid(x0, y0, cell, cols, rows, cells, [], rects.length);
    rects.forEach((r, i) => {
      if (!finite(r)) {
        grid.wild.push(i);
        return;
      }
      const [c0, c1, r0, r1] = grid.span(r, 0.0);
      for (let row = r0; row <= r1; row++) {
        for (let col = c0; col <= c1; col++) {
          grid.cells[row * cols + col]!.push(i);
        }
      }
    });
    return grid;
  }

  /// Column and row ranges of the cells that `r`, grown by `margin`, touches.
  private span(r: Rect, margin: number): [number, number, number, number] {
    const col = (x: number) => Math.min(asUsize(fmax((x - this.x0) / this.cell, 0.0)), this.cols - 1);
    const row = (y: number) => Math.min(asUsize(fmax((y - this.y0) / this.cell, 0.0)), this.rows - 1);
    return [
      col(fmin(r.x, r.right()) - margin),
      col(fmax(r.x, r.right()) + margin),
      row(fmin(r.y, r.bottom()) - margin),
      row(fmax(r.y, r.bottom()) + margin),
    ];
  }

  /// Indices, ascending, of every box that may lie within `margin` of `r`: a superset of
  /// the boxes that do.
  near(r: Rect, margin: number): number[] {
    if (![r.x, r.y, r.w, r.h, margin].every((v) => Number.isFinite(v))) {
      return Array.from({ length: this.len }, (_, i) => i);
    }
    const [c0, c1, r0, r1] = this.span(r, fmax(margin, 0.0));
    const out = [...this.wild];
    for (let row = r0; row <= r1; row++) {
      for (let col = c0; col <= c1; col++) {
        out.push(...this.cells[row * this.cols + col]!);
      }
    }
    if (out.length < Math.floor(this.len / 16)) {
      out.sort((a, b) => a - b);
      return out.filter((v, i) => i === 0 || out[i - 1] !== v);
    }
    const hit: boolean[] = new Array<boolean>(this.len).fill(false);
    for (const i of out) {
      hit[i] = true;
    }
    const res: number[] = [];
    for (let i = 0; i < this.len; i++) if (hit[i]) res.push(i);
    return res;
  }
}
