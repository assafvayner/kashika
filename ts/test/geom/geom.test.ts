import { describe, expect, test } from "vitest";
import {
  clipToRect,
  endDir,
  FLATTEN_TOLERANCE,
  flatten,
  isAxisAligned,
  length,
  line as seg,
  MAX_FLATTEN_STEPS,
  type PathSeg,
  pointAt,
  Pt,
  quad,
  cubic,
  Rect,
  roundCorners,
  roundCornersTagged,
  segEval,
  segLength,
  segTrim,
  startDir,
} from "../../src/geom/index.js";

function line(a: [number, number], b: [number, number]): PathSeg {
  return seg(new Pt(a[0], a[1]), new Pt(b[0], b[1]));
}

describe("geom", () => {
  test("rect_helpers", () => {
    const r = new Rect(10, 20, 100, 60);
    expect(r.center()).toEqual(new Pt(60, 50));
    expect([r.right(), r.bottom()]).toEqual([110, 80]);
  });

  test("length_and_point_at_follow_arc_length", () => {
    const path = [line([0, 0], [100, 0]), line([100, 0], [100, 100])];
    expect(Math.abs(length(path) - 200)).toBeLessThan(1e-9);
    expect(pointAt(path, 0.75)).toEqual(new Pt(100, 50));
  });

  test("quad_flattening_stays_within_tolerance", () => {
    const q = quad(new Pt(0, 0), new Pt(50, -100), new Pt(100, 0));
    const pts = flatten([q]);
    for (let i = 0; i + 1 < pts.length; i++) {
      const mid = pts[i]!.lerp(pts[i + 1]!, 0.5);
      const t = mid.x / 100;
      const exact = segEval(q, t);
      expect(Math.abs(exact.y - mid.y)).toBeLessThanOrEqual(FLATTEN_TOLERANCE + 1e-9);
    }
  });

  test("axis_alignment", () => {
    expect(isAxisAligned(line([0, 0], [0, 50]))).toBe(true);
    expect(isAxisAligned(line([0, 0], [10, 50]))).toBe(false);
  });

  test("rounds_polyline_corners_and_keeps_curves", () => {
    const path = [line([0, 0], [100, 0]), line([100, 0], [100, 100])];
    const r = roundCorners(path, 10);
    expect(r).toEqual([
      line([0, 0], [90, 0]),
      quad(new Pt(90, 0), new Pt(100, 0), new Pt(100, 10)),
      line([100, 10], [100, 100]),
    ]);
  });

  test("radius_is_clamped_to_half_the_shorter_segment", () => {
    const path = [line([0, 0], [10, 0]), line([10, 0], [10, 100])];
    const r = roundCorners(path, 50);
    expect(r[0]).toEqual(line([0, 0], [5, 0]));
  });

  test("directions_at_ends", () => {
    const q = quad(new Pt(0, 0), new Pt(0, -10), new Pt(10, -10));
    expect(startDir(q)).toEqual(new Pt(0, -1));
    expect(endDir(q)).toEqual(new Pt(1, 0));
  });

  test("flattening_huge_curves_is_bounded", () => {
    const c = cubic(new Pt(0, 0), new Pt(1e300, 0), new Pt(0, 1e300), new Pt(10, 10));
    expect(flatten([c]).length).toBeLessThanOrEqual(MAX_FLATTEN_STEPS + 1);
  });

  test("trim_follows_the_curve", () => {
    const q = quad(new Pt(0, 0), new Pt(50, -100), new Pt(100, 0));
    let t = segTrim(q, 0, 10);
    expect(t.kind).toBe("quad");
    expect(t.from).toEqual(q.from);
    expect(Math.abs(segLength(q) - segLength(t) - 10)).toBeLessThan(0.5);
    let onCurve = Infinity;
    for (let k = 0; k <= 1000; k++) {
      onCurve = Math.min(onCurve, segEval(q, k / 1000).dist(t.to));
    }
    expect(onCurve).toBeLessThan(1);

    t = segTrim(q, 10, 0);
    expect(t.to).toEqual(q.to);
    expect(Math.abs(segLength(q) - segLength(t) - 10)).toBeLessThan(0.5);
  });

  test("trim_splits_overlapping_ends_in_proportion", () => {
    let t = segTrim(line([0, 0], [15, 0]), 10, 10);
    expect(t).toEqual(line([7.5, 0], [7.5, 0]));
    t = segTrim(line([0, 0], [100, 0]), 10, 12);
    expect(t).toEqual(line([10, 0], [88, 0]));
  });

  test("rounding_keeps_segment_tags", () => {
    const path: [PathSeg, number][] = [
      [line([0, 0], [100, 0]), 0],
      [line([100, 0], [100, 100]), 1],
      [line([100, 100], [200, 100]), 1],
    ];
    const out = roundCornersTagged(path, 10);
    expect(out.map(([, t]) => t)).toEqual([0, 0, 1, 1, 1]);
    const plain = out.map(([s]) => s);
    const untagged = path.map(([s]) => s);
    expect(plain).toEqual(roundCorners(untagged, 10));
  });

  test("clip_to_rect_keeps_the_part_inside", () => {
    const tri = [new Pt(0, 0), new Pt(20, 0), new Pt(0, 20)];
    const r = new Rect(5, 0, 10, 10);
    const c = clipToRect(tri, r);
    expect(Rect.bounding(c)).toEqual(r);
    expect(c.every((p) => p.x >= 5 && p.x <= 15 && p.y <= 10)).toBe(true);
    expect(clipToRect(tri, new Rect(30, 30, 5, 5))).toEqual([]);
    expect(clipToRect(tri, new Rect(-1, -1, 30, 30)).length).toBe(3);
  });
});
