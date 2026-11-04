import { describe, expect, test } from "vitest";
import { Diagnostic, Element, Float, Span } from "../../src/diag/index.js";
import { compact, diagnostics, foldDebug, pretty } from "../../src/geom/dump.js";

describe("dump", () => {
  test("compact_rounds_floats_and_sorts_keys", () => {
    expect(compact({ b: new Float(1.0051), a: [1, new Float(2), 0.333] })).toBe(
      '{"a":[1,2,0.33],"b":1.01}',
    );
  });

  test("pretty_keeps_points_inline", () => {
    expect(pretty({ p: { x: new Float(1), y: new Float(2) }, n: [[{ a: 1 }, [1, 2]]] })).toBe(
      '{\n  "n": [\n    [{"a":1},[1,2]]\n  ],\n  "p": {"x":1,"y":2}\n}\n',
    );
    expect(pretty([])).toBe("[]\n");
  });

  test("diagnostics_print_one_block_each", () => {
    const span = Span.new(2, 1, 2, 9);
    const el = Element.new("a", "line", span);
    el.from = [0.123, 0];
    const d = Diagnostic.warn("cross", span, "this is kashika 0.1.0-dev0")
      .withHelp("h")
      .withElement(el)
      .withFact("at", [new Float(5), new Float(0)]);
    d.suppressed = true;
    expect(diagnostics([d.toJson()])).toBe(
      "warn[cross] 2:1-2:9: this is kashika <tool>\n" +
        "  = help: h\n" +
        "  = suppressed\n" +
        '  elements: [{"from":[0.12,0],"id":"a","kind":"line","span":{"col":1,"end_col":9,"end_line":2,"line":2}}]\n' +
        '  facts: {"at":[5,0]}\n',
    );
    expect(diagnostics([])).toBe("(no diagnostics)\n");
  });

  test("fold_debug_folds_leaf_groups_and_spans", () => {
    const text = [
      "File {",
      "    span: Span {",
      "        line: 4,",
      "        col: 1,",
      "        end_line: 4,",
      "        end_col: 33,",
      "    },",
      "    items: [",
      "        1,",
      "        2,",
      "    ],",
      "}",
    ].join("\n");
    expect(foldDebug(text)).toBe("File { span: 4:1-4:33, items: [1, 2] }\n");
  });
});
