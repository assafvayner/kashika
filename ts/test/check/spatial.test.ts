import { expect, it } from "vitest";

import { Grid } from "../../src/check/spatial.js";
import { fmax, fmin } from "../../src/diag/rust.js";
import { Rect } from "../../src/geom/index.js";

it("finds_nearby_boxes_in_order", () => {
  const rects: Rect[] = [];
  for (let i = 0; i < 100; i++) {
    rects.push(new Rect((i % 10) * 50.0, Math.floor(i / 10) * 50.0, 20.0, 20.0));
  }
  const grid = Grid.new(rects);
  const near = grid.near(new Rect(105.0, 105.0, 0.0, 0.0), 10.0);
  expect(near).toContain(22);
  expect(near.every((v, i) => i === 0 || near[i - 1]! < v)).toBe(true);
  expect(near).not.toContain(99);
  expect(grid.near(new Rect(NaN, 0.0, 0.0, 0.0), 0.0).length).toBe(100);
});

const MASK = (1n << 64n) - 1n;

/// A small deterministic generator, so the property test needs no dependency.
class Lcg {
  constructor(private s: bigint) {}

  next(lo: number, hi: number): number {
    this.s = (this.s * 6364136223846793005n + 1442695040888963407n) & MASK;
    return lo + (hi - lo) * (Number(this.s >> 11n) / 2 ** 53);
  }
}

function finite(r: Rect): boolean {
  return [r.x, r.y, r.w, r.h].every((v) => Number.isFinite(v));
}

function nearBrute(rects: Rect[], q: Rect, margin: number): number[] {
  const [qx0, qx1] = [fmin(q.x, q.right()), fmax(q.x, q.right())];
  const [qy0, qy1] = [fmin(q.y, q.bottom()), fmax(q.y, q.bottom())];
  const out: number[] = [];
  rects.forEach((r, i) => {
    if (
      !finite(r) ||
      (fmin(r.x, r.right()) - margin <= qx1 &&
        fmax(r.x, r.right()) + margin >= qx0 &&
        fmin(r.y, r.bottom()) - margin <= qy1 &&
        fmax(r.y, r.bottom()) + margin >= qy0)
    ) {
      out.push(i);
    }
  });
  return out;
}

it("grid_finds_everything_brute_force_finds", () => {
  const rng = new Lcg(7n);
  for (let round = 0; round < 50; round++) {
    const n = (round * 7) % 120;
    const rects: Rect[] = [];
    for (let i = 0; i < n; i++) {
      rects.push(
        new Rect(
          rng.next(-500.0, 1500.0),
          rng.next(-500.0, 1500.0),
          rng.next(-50.0, 300.0),
          rng.next(-50.0, 300.0),
        ),
      );
    }
    if (round % 5 === 0 && n > 0) {
      rects[Math.floor(n / 2)] = new Rect(NaN, 0.0, 10.0, 10.0);
    }
    const grid = Grid.new(rects);
    for (let k = 0; k < 40; k++) {
      const q = new Rect(
        rng.next(-700.0, 1700.0),
        rng.next(-700.0, 1700.0),
        rng.next(-100.0, 400.0),
        rng.next(-100.0, 400.0),
      );
      const margin = rng.next(0.0, 30.0);
      const got = grid.near(q, margin);
      expect(got.every((v, i) => i === 0 || got[i - 1]! < v)).toBe(true);
      expect(got.every((i) => i < rects.length)).toBe(true);
      for (const i of nearBrute(rects, q, margin)) {
        expect(got).toContain(i);
      }
    }
  }
});

it("an_empty_grid_finds_nothing", () => {
  const grid = Grid.new([]);
  expect(grid.near(new Rect(0.0, 0.0, 10.0, 10.0), 5.0)).toEqual([]);
  expect(grid.near(new Rect(NaN, 0.0, 0.0, 0.0), 0.0)).toEqual([]);
});

it("non_finite_rects_are_found_by_every_query", () => {
  const rects = [new Rect(NaN, NaN, 0.0, 0.0), new Rect(0.0, 0.0, Infinity, 1.0)];
  const grid = Grid.new(rects);
  expect(grid.near(new Rect(5.0, 5.0, 1.0, 1.0), 0.0)).toEqual([0, 1]);
});
