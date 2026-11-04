import { describe, expect, it } from "vitest";
import { Span } from "../../src/diag/index.js";
import { SourceText } from "../../src/syntax/index.js";

describe("source", () => {
  it("slices_spans_by_char", () => {
    const s = SourceText.new('\u{feff}box a "é"\nline x: a -> b\n');
    expect(s.span(new Span(1, 7, 1, 10))).toBe('"é"');
    expect(s.span(new Span(2, 1, 2, 5))).toBe("line");
    expect(s.between(new Span(1, 5, 1, 6), new Span(2, 1, 2, 5))).toBe('a "é" line');
  });

  it("leaves_out_what_lies_outside_the_source", () => {
    const s = SourceText.new("box a\n");
    expect(s.span(new Span(1, 5, 1, 99))).toBe("a");
    expect(s.span(new Span(9, 1, 9, 4))).toBe("");
    expect(s.span(new Span(1, 9, 1, 12))).toBe("");
    expect(s.span(Span.default())).toBe("");
  });
});
