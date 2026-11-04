import { describe, expect, test } from "vitest";
import {
  type Block,
  type Run,
  colorLayers,
  measure,
  onlyEmoji,
  outline,
  parseMarkup,
} from "../../src/text/index.js";

function runs(s: string): Run[] {
  const r = parseMarkup(s, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value[0]!;
}

function m(s: string, size = 14): Block {
  const r = measure(s, size, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value;
}

function width(s: string): number {
  return m(s).width();
}

describe("fallback", () => {
  test("box_drawing_falls_back_to_the_mono_face", () => {
    const b = m("┌─┐│└┘");
    expect(b.missingGlyphs()).toEqual([]);
    expect(b.width()).toBe(width("`┌─┐│└┘`"));
    expect(Math.abs(b.width() - 6 * 0.6 * 14)).toBeLessThan(1e-9);
    expect(outline(runs("┌─┐"), 14, 0, 0)).not.toBe("");
    expect(colorLayers(runs("┌─┐"), 14, 0, 0)).toEqual([]);
  });

  test("code_falls_back_to_inter_and_bold_to_mono", () => {
    const code = m("`₹`");
    expect(code.missingGlyphs()).toEqual([]);
    expect(code.width()).toBe(width("₹"));
    const bold = m("**─**");
    expect(bold.missingGlyphs()).toEqual([]);
    expect(bold.width()).toBe(width("`─`"));
  });

  test("characters_no_face_covers_are_still_missing", () => {
    expect(m("日 한 ✘ 🚀 ┌").missingGlyphs()).toEqual(["日", "한", "✘"]);
  });

  test("emoji_advance_one_em_in_any_run", () => {
    for (const s of ["🚀", "✅", "🔥", "✔", "**🚀**", "`🚀`"]) {
      expect(width(s), s).toBe(14);
      expect(m(s).missingGlyphs(), s).toEqual([]);
    }
    expect(width("a🚀b")).toBe(width("ab") + 14);
    expect(m("🚀", 28).width()).toBe(28);
  });

  test("sequences_form_one_ligature_glyph", () => {
    for (const s of [
      "👍🏽",
      "👩\u{200D}💻",
      "🇺🇸",
      "1\u{FE0F}\u{20E3}",
      "#\u{FE0F}\u{20E3}",
      "⚠\u{FE0F}",
      "❤\u{FE0F}",
    ]) {
      expect(width(s), s).toBe(14);
      expect(outline(runs(s), 14, 0, 0), s).toBe("");
    }
    const layers = (s: string) => colorLayers(runs(s), 14, 0, 0);
    expect(layers("👍🏽")).not.toEqual(layers("👍"));
    expect(layers("👩\u{200D}💻")).not.toEqual(layers("👩"));
    expect(width("🇺🇸🇺")).toBe(28);
  });

  test("text_presentation_stays_in_the_text_face", () => {
    for (const s of ["⚠", "⚠\u{FE0E}", "1", "1\u{FE0F}", "©"]) {
      expect(colorLayers(runs(s), 14, 0, 0), s).toEqual([]);
      expect(outline(runs(s), 14, 0, 0), s).not.toBe("");
    }
    expect(width("1\u{FE0F}")).toBe(width("1"));
  });

  test("emoji_draw_as_colored_layers_after_the_pen", () => {
    const l = colorLayers(runs("ab🚀"), 14, 10, 20);
    expect(l.length).toBeGreaterThan(0);
    expect(l.every((l) => l.color !== null && l.d.startsWith("M"))).toBe(true);
    expect(l).toEqual(colorLayers(runs("ab🚀"), 14, 10, 20));
    const shifted = colorLayers(runs("🚀"), 14, 10 + width("ab"), 20);
    expect(l).toEqual(shifted);
    expect(outline(runs("ab🚀"), 14, 10, 20)).toBe(outline(runs("ab"), 14, 10, 20));
  });

  test("only_emoji_ignores_spaces_and_needs_an_emoji", () => {
    expect(onlyEmoji(runs("🚀 ✅"))).toBe(true);
    expect(onlyEmoji(runs("🚀 a"))).toBe(false);
    expect(onlyEmoji(runs(" "))).toBe(false);
    expect(onlyEmoji(runs("┌"))).toBe(false);
  });
});
