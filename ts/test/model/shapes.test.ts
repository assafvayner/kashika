import { describe, expect, test } from "vitest";
import { Rect } from "../../src/geom/index.js";
import { builtinSource } from "../../src/model/index.js";

describe("shapes", () => {
  test("builtin_source_is_exposed_for_ref", () => {
    const [d, outline, ta] = builtinSource("diamond", 120.0, 80.0)!;
    expect(d).toBe("M 0.5 0 L 1 0.5 L 0.5 1 L 0 0.5 Z");
    expect(outline).toBe(1);
    expect(ta.eq(new Rect(0.22, 0.28, 0.56, 0.44))).toBe(true);
    expect(builtinSource("rect", 1.0, 1.0)).toBeNull();
  });
});
