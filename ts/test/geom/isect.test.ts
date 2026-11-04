import { describe, expect, test } from "vitest";
import {
  boundaryNearest,
  distPointSeg,
  insideRuns,
  line,
  pieces,
  pointInPolygon,
  polygonsOverlap,
  Pt,
  quad,
  Rect,
  rectPolygon,
  segHit,
  segSegDist,
  strictlyInside,
} from "../../src/geom/index.js";

function p(x: number, y: number): Pt {
  return new Pt(x, y);
}

describe("isect", () => {
  test("crossing_segments_meet_at_one_point", () => {
    expect(segHit(p(0, 0), p(10, 10), p(0, 10), p(10, 0), 0.5)).toEqual({
      kind: "point",
      at: p(5, 5),
    });
  });

  test("touching_within_tolerance_counts", () => {
    expect(segHit(p(0, 0), p(10, 0), p(5, 0.3), p(5, 10), 0.5)).toEqual({
      kind: "point",
      at: p(5, 0.3),
    });
    expect(segHit(p(0, 0), p(10, 0), p(5, 1), p(5, 10), 0.5)).toBeNull();
  });

  test("collinear_overlap_and_parallel_miss", () => {
    expect(segHit(p(0, 0), p(10, 0), p(5, 0), p(20, 0), 0.5)).toEqual({
      kind: "overlap",
      from: p(5, 0),
      to: p(10, 0),
    });
    expect(segHit(p(0, 0), p(10, 0), p(0, 3), p(10, 3), 0.5)).toBeNull();
  });

  test("distances", () => {
    expect(distPointSeg(p(5, 5), p(0, 0), p(10, 0))).toBe(5);
    expect(segSegDist(p(0, 0), p(10, 0), p(0, 4), p(10, 4))).toBe(4);
    expect(segSegDist(p(0, 0), p(10, 10), p(0, 10), p(10, 0))).toBe(0);
  });

  test("boundary_distance_and_inside", () => {
    const sq = rectPolygon(new Rect(0, 0, 10, 10));
    expect(boundaryNearest(p(5, 3), sq)).toEqual([3, p(5, 0)]);
    expect(strictlyInside(p(5, 5), sq)).toBe(true);
    expect(strictlyInside(p(5, 0.2), sq)).toBe(false);
    expect(pointInPolygon(p(15, 5), sq)).toBe(false);
  });

  test("inside_runs_clip_to_the_interior", () => {
    const sq = rectPolygon(new Rect(10, 10, 20, 20));
    expect(insideRuns(p(0, 20), p(40, 20), sq)).toEqual([[p(10, 20), p(30, 20)]]);
    expect(insideRuns(p(0, 10), p(40, 10), sq)).toEqual([]);
    expect(insideRuns(p(0, 0), p(5, 40), sq)).toEqual([]);
    expect(insideRuns(p(20, 20), p(40, 20), sq)).toEqual([[p(20, 20), p(30, 20)]]);
  });

  test("polygon_overlap_ignores_touching", () => {
    const a = rectPolygon(new Rect(0, 0, 10, 10));
    expect(polygonsOverlap(a, rectPolygon(new Rect(10, 0, 10, 10)))).toBe(false);
    expect(polygonsOverlap(a, rectPolygon(new Rect(5, 5, 10, 10)))).toBe(true);
    expect(polygonsOverlap(a, rectPolygon(new Rect(0, 0, 10, 10)))).toBe(true);
    expect(polygonsOverlap(a, rectPolygon(new Rect(2, 2, 3, 3)))).toBe(true);
  });

  test("pieces_flatten_curves_and_keep_indices", () => {
    const path = [line(p(0, 0), p(10, 0)), quad(p(10, 0), p(20, 0), p(20, 10))];
    const ps = pieces(path);
    expect(ps[0]).toEqual([p(0, 0), p(10, 0), 0]);
    expect(ps.length > 2 && ps.slice(1).every((x) => x[2] === 1)).toBe(true);
    expect(ps[ps.length - 1]![1]).toEqual(p(20, 10));
  });
});
