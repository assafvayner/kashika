import { describe, expect, test } from "vitest";
import {
  Diagnostic,
  didYouMean,
  Element,
  fact,
  Float,
  sort,
  Span,
  toJsonString,
} from "../../src/diag/index.js";

describe("kashika-diag", () => {
  test("suggests_close_names", () => {
    expect(didYouMean("rigth", ["right", "left"])).toBe("right");
    expect(didYouMean("bx", ["box", "text", "line"])).toBe("box");
  });

  test("suggests_real_typos", () => {
    expect(didYouMean("boundry", ["boundary", "box"])).toBe("boundary");
    expect(didYouMean("pinn", ["pin"])).toBe("pin");
    expect(didYouMean("colr", ["color"])).toBe("color");
    expect(didYouMean("stroke-widht", ["stroke-width"])).toBe("stroke-width");
  });

  test("does_not_suggest_loose_matches", () => {
    expect(didYouMean("icon", ["cdn"])).toBeNull();
    expect(didYouMean("condition", ["function"])).toBeNull();
    expect(didYouMean("centre", ["center"])).toBe("center");
    expect(didYouMean("abcdef", ["abcxyz"])).toBeNull();
  });

  test("does_not_suggest_distant_names", () => {
    expect(didYouMean("zzzzz", ["right"])).toBeNull();
  });

  test("never_suggests_the_name_itself", () => {
    expect(didYouMean("right", ["right"])).toBeNull();
    expect(didYouMean("right", ["right", "rigt"])).toBe("rigt");
  });

  test("never_suggests_one_letter_names", () => {
    expect(didYouMean("c", ["a", "b"])).toBeNull();
    expect(didYouMean("ab", ["a"])).toBeNull();
    expect(didYouMean("x", ["xy"])).toBe("xy");
  });

  test("suggests_nothing_without_candidates", () => {
    expect(didYouMean("box", [])).toBeNull();
    expect(didYouMean("", ["a", "ab"])).toBeNull();
  });

  test("ties_go_to_the_alphabetically_first_candidate", () => {
    expect(didYouMean("cat", ["cot", "bat"])).toBe("bat");
  });

  test("span_to_joins_start_and_end", () => {
    const a = Span.new(2, 3, 2, 9);
    const b = Span.new(4, 1, 5, 7);
    expect(a.to(b)).toEqual(Span.new(2, 3, 5, 7));
    expect(a.to(a)).toEqual(a);
  });

  test("sorts_by_position_then_severity", () => {
    const d = [
      Diagnostic.error("size", Span.new(3, 1, 3, 2), "x"),
      Diagnostic.error("attr", Span.new(1, 5, 1, 6), "y"),
    ];
    sort(d);
    expect(d[0]!.code).toBe("attr");
  });

  test("sorts_errors_before_warnings_then_by_code_end_and_message", () => {
    const at = Span.new(1, 1, 1, 5);
    const d = [
      Diagnostic.warn("near", at, "b"),
      Diagnostic.warn("near", at, "a"),
      Diagnostic.warn("near", Span.new(1, 1, 1, 3), "c"),
      Diagnostic.warn("cross", at, "z"),
      Diagnostic.error("parse", at, "e"),
    ];
    sort(d);
    expect(d.map((x) => [x.code, x.message])).toEqual([
      ["parse", "e"],
      ["cross", "z"],
      ["near", "c"],
      ["near", "a"],
      ["near", "b"],
    ]);
  });

  test("warnings_need_a_lint_code", () => {
    expect(() => Diagnostic.warn("parse", Span.default(), "x")).toThrow("not a lint code");
  });

  test("errors_need_an_error_code", () => {
    expect(() => Diagnostic.error("cross", Span.default(), "x")).toThrow("not an error code");
  });

  test("warnings_serialize_elements_facts_and_suppression", () => {
    const span = Span.new(2, 1, 2, 9);
    const el = Element.new("a", "line", span);
    el.segment = 1;
    el.from = [0, 0];
    el.to = [10, 0];
    const d = Diagnostic.warn("cross", span, "a crosses b")
      .withElement(el)
      .withFact("at", [new Float(5), new Float(0)]);
    d.suppressed = true;
    d.reason = "bus";
    const s = toJsonString(d.toJson());
    const v = JSON.parse(s);
    expect(v.severity).toBe("warn");
    expect(v.elements[0].id).toBe("a");
    expect(v.elements[0].segment).toBe(1);
    expect(v.elements[0].bbox).toBeUndefined();
    expect(v.facts.at).toEqual([5.0, 0.0]);
    expect(s).toContain('"facts":{"at":[5.0,0.0]}');
    expect(s).toContain('"from":[0.0,0.0]');
    expect(d.point(fact.AT)).toEqual([5.0, 0.0]);
    expect(d.point(fact.ENTRY)).toBeNull();
    expect(v.suggestions).toEqual([]);
    expect(v.suppressed).toBe(true);
    expect(v.reason).toBe("bus");

    const e = JSON.parse(toJsonString(Diagnostic.error("parse", Span.default(), "x").toJson()));
    expect(e.elements).toBeUndefined();
    expect(e.facts).toBeUndefined();
    expect(e.suppressed).toBeUndefined();
    expect(e.reason).toBeUndefined();
  });
});
