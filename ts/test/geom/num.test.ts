import { describe, expect, test } from "vitest";
import { displayF64 } from "../../src/diag/index.js";
import { fmtNum, round2 } from "../../src/geom/index.js";

describe("num", () => {
  test("rounds_to_two_decimals_without_negative_zero", () => {
    expect(fmtNum(1.0051)).toBe("1.01");
    expect(fmtNum(12.0)).toBe("12");
    expect(fmtNum(-0.001)).toBe("0");
    expect(fmtNum(-2.5)).toBe("-2.5");
    expect(round2(-0.004)).toBe(0);
    expect(Object.is(round2(-0.004), 0)).toBe(true);
  });

  test("writes_non_finite_and_huge_values_as_they_are", () => {
    expect(fmtNum(Infinity)).toBe("inf");
    expect(fmtNum(NaN)).toBe("NaN");
    expect(fmtNum(Number.MAX_VALUE)).toBe(displayF64(Number.MAX_VALUE));
  });
});
