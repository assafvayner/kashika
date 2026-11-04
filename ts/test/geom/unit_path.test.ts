import { describe, expect, test } from "vitest";
import {
  arcToCubics,
  convexHull,
  horizontalHits,
  line,
  type PathSeg,
  parseUnitPath,
  Pt,
  Rect,
  segEval,
  segMap,
  verticalHits,
} from "../../src/geom/index.js";
import { totalCmp } from "../../src/diag/index.js";

function unwrap(d: string): PathSeg[][] {
  const r = parseUnitPath(d);
  if (!r.ok) throw new Error(r.error);
  return r.value;
}

describe("unit_path", () => {
  test("parses_lines_and_close", () => {
    const sp = unwrap("M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z");
    expect(sp.length).toBe(1);
    expect(sp[0]!.length).toBe(4);
    expect(sp[0]![3]).toEqual(line(new Pt(0, 0.5), new Pt(0.5, 0)));
  });

  test("h_v_q_c_and_implicit_lineto", () => {
    const sp = unwrap("M 0,0 1,0 V 1 H 0.5 Q 0.25 1 0 0.5 C 0 0.3 0.1 0 0 0");
    const s = sp[0]!;
    expect(s.length).toBe(5);
    expect(s[0]).toEqual(line(new Pt(0, 0), new Pt(1, 0)));
    expect(s[1]!.to).toEqual(new Pt(1, 1));
    expect(s[2]!.to).toEqual(new Pt(0.5, 1));
    expect(s[3]!.kind).toBe("quad");
    expect(s[4]!.kind).toBe("cubic");
  });

  test("multiple_subpaths", () => {
    const sp = unwrap("M 0 0 L 1 1  M 0 1 L 1 0");
    expect(sp.length).toBe(2);
    expect(sp[1]![0]!.from).toEqual(new Pt(0, 1));
  });

  test("arcs_become_cubics_on_the_requested_side", () => {
    let sp = unwrap("M 0 0.5 A 0.5 0.5 0 0 1 1 0.5");
    expect(sp[0]!.length).toBe(2);
    expect(sp[0]!.every((s) => s.kind === "cubic")).toBe(true);
    expect(sp[0]![0]!.to.dist(new Pt(0.5, 0)), "sweep=1 goes over the top on screen").toBeLessThan(
      1e-9,
    );
    expect(sp[0]![1]!.to.dist(new Pt(1, 0.5))).toBeLessThan(1e-12);
    sp = unwrap("M 0 0.5 A 0.5 0.5 0 0 0 1 0.5");
    expect(sp[0]![0]!.to.dist(new Pt(0.5, 1)), "sweep=0 goes under").toBeLessThan(1e-9);
  });

  test("arc_stays_close_to_the_circle", () => {
    const segs = arcToCubics(new Pt(0, 50), 50, 50, 0, false, true, new Pt(100, 50));
    for (const s of segs) {
      for (let k = 0; k <= 10; k++) {
        const p = segEval(s, k / 10);
        expect(Math.abs(p.dist(new Pt(50, 50)) - 50)).toBeLessThan(0.05);
      }
    }
  });

  test("small_radii_are_scaled_up", () => {
    const segs = arcToCubics(new Pt(0, 0), 0.1, 0.1, 0, false, true, new Pt(1, 0));
    expect(segs[segs.length - 1]!.to.dist(new Pt(1, 0))).toBeLessThan(1e-12);
    expect(segs[0]!.to.dist(new Pt(0.5, -0.5))).toBeLessThan(1e-9);
  });

  test("rejects_bad_paths", () => {
    const cases: [string, string][] = [
      ["", "empty"],
      ["L 1 1", "must start with `M`"],
      ["m 0 0", "relative"],
      ["M 0 0 X 1", "unknown path command"],
      ["M 0 0 L 1.5 0", "between 0 and 1"],
      ["M 0 0 L 1", "expects 2 numbers"],
      ["M 0 0 A 1 1 0 2 0 1 1", "flags"],
      ["M 0 0 Z 1 1", "numbers must follow a command"],
      ["M 0 0 L 0.5.5 1", "bad number"],
    ];
    for (const [d, msg] of cases) {
      const r = parseUnitPath(d);
      expect(r.ok, d).toBe(false);
      if (!r.ok) expect(r.error, d).toContain(msg);
    }
  });

  test("unit_mapping", () => {
    const r = new Rect(100, 50, 200, 100);
    expect(r.mapUnit(new Pt(0.5, 0.25))).toEqual(new Pt(200, 75));
    expect(r.mapRect(new Rect(0.25, 0.5, 0.5, 0.5))).toEqual(new Rect(150, 100, 100, 50));
    const s = segMap(line(new Pt(0, 0), new Pt(1, 1)), (p) => r.mapUnit(p));
    expect(s.to).toEqual(new Pt(300, 150));
  });

  test("hull_drops_interior_and_duplicate_points", () => {
    const pts = [
      new Pt(0, 0),
      new Pt(1, 0),
      new Pt(1, 1),
      new Pt(0, 1),
      new Pt(0.5, 0.5),
      new Pt(1, 1),
    ];
    const h = convexHull(pts);
    expect(h.length).toBe(4);
    for (const p of [new Pt(0, 0), new Pt(1, 0), new Pt(1, 1), new Pt(0, 1)]) {
      expect(h.some((q) => q.eq(p))).toBe(true);
    }
  });

  test("hull_of_collinear_points_is_its_endpoints", () => {
    const h = convexHull([new Pt(0, 0), new Pt(1, 1), new Pt(2, 2)]);
    expect(h).toEqual([new Pt(0, 0), new Pt(2, 2)]);
  });

  test("axis_hits_on_a_diamond", () => {
    const d = [new Pt(60, 0), new Pt(120, 40), new Pt(60, 80), new Pt(0, 40)];
    const ys = verticalHits(d, 30).sort(totalCmp);
    expect(ys).toEqual([20, 60]);
    const xs = horizontalHits(d, 20).sort(totalCmp);
    expect(xs).toEqual([30, 90]);
    expect(verticalHits(d, 130)).toEqual([]);
  });
});
