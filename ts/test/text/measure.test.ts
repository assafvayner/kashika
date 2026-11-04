import { describe, expect, test } from "vitest";
import {
  type Block,
  type Run,
  coveredScripts,
  isDefaultIgnorable,
  measure,
  outline,
  parseMarkup,
} from "../../src/text/index.js";

function m(s: string, size: number, bold: boolean): Block {
  const r = measure(s, size, bold);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value;
}

function lines(s: string): Run[][] {
  const r = parseMarkup(s, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value;
}

describe("measure", () => {
  test("widths_come_from_the_bundled_fonts", () => {
    const b = m("API", 14, false);
    expect(Math.abs(b.width() - 22.3604)).toBeLessThan(1e-3);
    expect(b.height()).toBe(17.5);
    const bold = m("API", 14, true);
    expect(Math.abs(bold.width() - 23.4541)).toBeLessThan(1e-3);
    const code = m("`API`", 14, false);
    expect(Math.abs(code.width() - 25.2)).toBeLessThan(1e-9);
  });

  test("width_scales_with_size_and_lines_stack", () => {
    const a = m("Postgres", 14, false);
    const b = m("Postgres", 28, false);
    expect(Math.abs(b.width() - 2 * a.width())).toBeLessThan(1e-9);
    const two = m("Postgres\nAPI", 12, false);
    expect(Math.abs(two.width() - 50.3262)).toBeLessThan(1e-3);
    expect(two.height()).toBe(30);
    expect(two.lines().length).toBe(2);
  });

  test("missing_glyphs_use_notdef_width", () => {
    const b = m("\u{10FFFD}", 14, false);
    expect(Math.abs(b.width() - (1344 * 14) / 2048)).toBeLessThan(1e-9);
  });

  test("unclosed_markup_fails_measurement", () => {
    const r = measure("**API", 14, false);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.marker).toBe("**");
  });

  test("outline_is_deterministic_path_data", () => {
    const runs = lines("Hi")[0]!;
    const d = outline(runs, 14, 10, 20);
    expect(d.startsWith("M")).toBe(true);
    expect(d).toContain("Z");
    expect(d).toBe(outline(runs, 14, 10, 20));
    expect(outline(lines(" ")[0]!, 14, 0, 0)).toBe("");
  });

  test("missing_glyphs_lists_each_character_once_in_order", () => {
    const b = m("a 日 **本日** `é`", 14, false);
    expect(b.missingGlyphs()).toEqual(["日", "本"]);
    expect(m("plain é ü", 14, false).missingGlyphs()).toEqual([]);
  });

  test("tabs_measure_as_four_spaces_and_are_not_missing", () => {
    const t = m("a\tb", 14, false);
    const s = m("a    b", 14, false);
    expect(t.width()).toBe(s.width());
    expect(t.missingGlyphs()).toEqual([]);
  });

  test("default_ignorable_characters_are_zero_width_and_never_missing", () => {
    const plain = m("ab", 14, false);
    for (const c of [
      "\u{200D}",
      "\u{200C}",
      "\u{FE0F}",
      "\u{E0100}",
      "\u{200B}",
      "\u{2060}",
      "\u{FEFF}",
      "\u{00AD}",
    ]) {
      expect(isDefaultIgnorable(c), c).toBe(true);
      const text = `a${c}b`;
      const b = m(text, 14, false);
      expect(b.width(), c).toBe(plain.width());
      expect(b.missingGlyphs(), c).toEqual([]);
      const runs = lines(text)[0]!;
      const ab = lines("ab")[0]!;
      expect(outline(runs, 14, 0, 0)).toBe(outline(ab, 14, 0, 0));
    }
    expect(isDefaultIgnorable("a")).toBe(false);
    expect(isDefaultIgnorable("\u{2010}")).toBe(false);
  });

  test("covered_scripts_come_from_the_fonts", () => {
    expect(coveredScripts()).toEqual(["Latin", "Greek", "Cyrillic", "box drawing", "emoji"]);
  });
});
