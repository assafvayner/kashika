import { describe, expect, test } from "vitest";
import { Pt, Rect } from "../../src/geom/index.js";

describe("rect", () => {
  test("from_points_takes_either_corner_order", () => {
    const r = new Rect(1, 2, 3, 4);
    expect(Rect.fromPoints(new Pt(4, 6), new Pt(1, 2))).toEqual(r);
    expect(Rect.bounding([new Pt(4, 2), new Pt(1, 6)])).toEqual(r);
    expect(Rect.bounding([])).toEqual(Rect.default());
  });

  test("union_intersect_and_contain", () => {
    const a = new Rect(0, 0, 10, 10);
    const b = new Rect(5, 5, 10, 10);
    expect(a.union(b)).toEqual(new Rect(0, 0, 15, 15));
    expect(a.intersect(b)).toEqual(new Rect(5, 5, 5, 5));
    expect(a.intersect(new Rect(10, 0, 5, 5))).toBeNull();
    expect(a.containsRect(new Rect(0, 0, 10, 10))).toBe(true);
    expect(a.containsRect(b)).toBe(false);
  });
});
