import { expect, it } from "vitest";

import { toJsonString } from "../../src/diag/json.js";
import { lint } from "./common/mod.js";

it("missing_glyph_flags_each_text_kind", () => {
  const d = lint(
    "missing-glyph",
    'box a "日本" at (20,20) size (200,80) sublabel="ok"\n' +
      'box b "fine é" at (200,20) size (120,40)\n' +
      'text t "x 日 **日** y" at (20,120)\n' +
      'line l a.right -> b.left label="→ 日"\n',
  );
  expect(d.length).toBe(3);
  expect(d[0]!.span.line).toBe(2);
  expect(toJsonString(d[0]!.facts.get("codepoints")!)).toBe('["U+65E5","U+672C"]');
  expect(d[1]!.elements[0]!.kind).toBe("text");
  expect(toJsonString(d[1]!.facts.get("codepoints")!)).toBe('["U+65E5"]');
  expect(d[2]!.elements[0]!.kind).toBe("line");
});

it("missing_glyph_is_quiet_for_covered_text_and_tabs", () => {
  expect(
    lint(
      "missing-glyph",
      'box a "Café Ünï Ωmega Привет" at (20,20) size (240,60)\n' +
        'text t "a\tb" at (20,120)\n' +
        'text u "zero​width‍join️soft­hyphen﻿" at (20,160)\n' +
        'text e "🚀 👍🏽 👩‍💻 🇺🇸 1️⃣ ✔" at (20,200)\n' +
        'text f "`┌─┐` ┌─┐ **└┘**" at (20,240)\n',
    ),
  ).toEqual([]);
});

it("missing_glyph_help_names_the_covered_sets", () => {
  const d = lint("missing-glyph", 'text t "日 ✘" at (20,20)\n');
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("codepoints")!)).toBe('["U+65E5","U+2718"]');
  expect(d[0]!.help.some((h) => h.includes("Cyrillic, box drawing or emoji"))).toBe(true);
});
