import { describe, expect, test } from "vitest";
import { type Result } from "../../src/diag/rust.js";
import { Pt, Rect } from "../../src/geom/index.js";
import { resolve } from "../../src/model/anchor.js";

const R = new Rect(0.0, 0.0, 100.0, 60.0);

function at(r: Result<Pt, [string, string | null]>): Pt {
  if (!r.ok) throw new Error(r.error[0]);
  return r.value;
}

describe("anchor", () => {
  test("rect_anchors", () => {
    expect(at(resolve(R, 0.0, "r", "top", null))).toEqual(new Pt(50.0, 0.0));
    expect(at(resolve(R, 0.0, "r", "right", 0.25))).toEqual(new Pt(100.0, 15.0));
    expect(at(resolve(R, 0.0, "r", "bottom-right", null))).toEqual(new Pt(100.0, 60.0));
  });

  test("rounded_anchors_are_axis_projected", () => {
    expect(at(resolve(R, 8.0, "r", "top", 0.0))).toEqual(new Pt(0.0, 8.0));
    expect(at(resolve(R, 8.0, "r", "top", null))).toEqual(new Pt(50.0, 0.0));
    expect(at(resolve(R, 8.0, "r", "tl", null))).toEqual(new Pt(0.0, 0.0));
  });

  test("anchor_errors", () => {
    const e = resolve(R, 0.0, "r", "rigth", null);
    expect(e.ok ? null : e.error[1]).toBe("did you mean `right`?");
    expect(resolve(R, 0.0, "r", "center", 0.5).ok).toBe(false);
    expect(resolve(R, 0.0, "r", "top", 1.5).ok).toBe(false);
  });
});
