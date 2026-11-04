import { describe, expect, test } from "vitest";
import {
  cmpStr,
  cmpTuple,
  debugF64,
  displayF64,
  Float,
  hypot,
  jsonString,
  parseF64,
  round,
  ryuF64,
  sum,
  toJsonString,
  totalCmp,
} from "../../src/diag/index.js";

// Expected strings are the output of rustc 1.97 and serde_json 1.0.154.
const FORMATS: [number, string, string, string][] = [
  [0, "0", "0.0", "0.0"],
  [-0, "-0", "-0.0", "-0.0"],
  [1, "1", "1.0", "1.0"],
  [12, "12", "12.0", "12.0"],
  [12.5, "12.5", "12.5", "12.5"],
  [-2.5, "-2.5", "-2.5", "-2.5"],
  [0.1, "0.1", "0.1", "0.1"],
  [1e21, "1000000000000000000000", "1e21", "1e+21"],
  [-1e21, "-1000000000000000000000", "-1e21", "-1e+21"],
  [1.5e-7, "0.00000015", "1.5e-7", "1.5e-7"],
  [1e16, "10000000000000000", "1e16", "1e+16"],
  [1e15, "1000000000000000", "1000000000000000.0", "1000000000000000.0"],
  [1.234e16, "12340000000000000", "1.234e16", "1.234e+16"],
  [123456789012345680, "123456789012345680", "1.2345678901234568e17", "1.2345678901234568e+17"],
  [0.00001, "0.00001", "1e-5", "0.00001"],
  [-1e-5, "-0.00001", "-1e-5", "-0.00001"],
  [0.0001, "0.0001", "0.0001", "0.0001"],
  [0.000012345, "0.000012345", "1.2345e-5", "0.000012345"],
  [1e-6, "0.000001", "1e-6", "1e-6"],
  [1e-7, "0.0000001", "1e-7", "1e-7"],
  [1.23e-18, "0.00000000000000000123", "1.23e-18", "1.23e-18"],
  [100, "100", "100.0", "100.0"],
  [2 / 3, "0.6666666666666666", "0.6666666666666666", "0.6666666666666666"],
  [9007199254740992, "9007199254740992", "9007199254740992.0", "9007199254740992.0"],
  [5e-324, `0.${"0".repeat(323)}5`, "5e-324", "5e-324"],
  [1.7976931348623157e308, `17976931348623157${"0".repeat(292)}`, "1.7976931348623157e308", "1.7976931348623157e+308"],
  [Infinity, "inf", "inf", "null"],
  [-Infinity, "-inf", "-inf", "null"],
  [NaN, "NaN", "NaN", "null"],
];

describe("rust compat", () => {
  test.each(FORMATS)("formats %s", (v, display, debug, json) => {
    expect(displayF64(v)).toBe(display);
    expect(debugF64(v)).toBe(debug);
    expect(ryuF64(v)).toBe(json);
  });

  test("json_strings_escape_like_serde_json", () => {
    expect(jsonString('a"b\\c\n\r\t\b\f\u0001\u001f\u007fé𝄞 /')).toBe(
      '"a\\"b\\\\c\\n\\r\\t\\b\\f\\u0001\\u001f\u007fé𝄞 /"',
    );
  });

  test("json_objects_sort_keys_and_mark_floats", () => {
    const m = new Map<string, null | number>([
      ["b", 1],
      ["a", null],
    ]);
    expect(toJsonString({ z: [1, new Float(1), 1.5], "￿": true, "𝄞": false, m })).toBe(
      '{"m":{"a":null,"b":1},"z":[1,1.0,1.5],"￿":true,"𝄞":false}',
    );
  });

  test("strings_compare_by_code_point", () => {
    expect(cmpStr("𝄞", "￿")).toBe(1);
    expect(cmpStr("a", "b")).toBe(-1);
    expect(cmpStr("ab", "a")).toBe(1);
    expect(cmpStr("", "")).toBe(0);
    expect(cmpTuple([1, "b"], [1, "a"])).toBe(1);
    expect(cmpTuple([false, 2], [true, 1])).toBe(-1);
  });

  test("total_cmp_orders_zeros_and_nan", () => {
    const v = [NaN, 1, -0, 0, -Infinity, -1].sort(totalCmp);
    expect(v.map((x) => (Object.is(x, -0) ? "-0" : String(x)))).toEqual([
      "-Infinity",
      "-1",
      "-0",
      "0",
      "1",
      "NaN",
    ]);
  });

  test("round_goes_away_from_zero_and_sum_starts_at_negative_zero", () => {
    expect(round(-2.5)).toBe(-3);
    expect(round(2.5)).toBe(3);
    expect(round(0.49999999999999994)).toBe(0);
    expect(Object.is(round(-0.3), -0)).toBe(true);
    expect(Object.is(sum([]), -0)).toBe(true);
  });

  test("hypot_matches_libm", () => {
    expect(hypot(9.210587170613312, -2.132198020115641)).toBe(9.454162280416577);
    expect(hypot(6.121331379220547, -3.231830538859095)).toBe(6.922096971737134);
    expect(hypot(3, 4)).toBe(5);
    expect(hypot(Infinity, NaN)).toBe(Infinity);
    expect(hypot(NaN, 1)).toBeNaN();
    expect(hypot(1.7976931348623157e308, 1.7976931348623157e308)).toBe(Infinity);
  });

  test("parses_floats_like_rust", () => {
    expect(parseF64("1.")).toBe(1);
    expect(parseF64("+.5")).toBe(0.5);
    expect(parseF64("infinity")).toBe(Infinity);
    expect(parseF64("1e")).toBeNull();
    expect(parseF64(".")).toBeNull();
    expect(parseF64("0.5.5")).toBeNull();
  });
});
