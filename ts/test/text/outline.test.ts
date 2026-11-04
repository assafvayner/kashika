import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";
import { baselineOffset, runWidth } from "../../src/text/font.js";
import { type Run, measure, outline, parseMarkup } from "../../src/text/index.js";

const RENDER = join(import.meta.dirname, "../../../tests/golden/render");

function pathData(file: string, label: string): string {
  const svg = readFileSync(join(RENDER, file), "utf8");
  const at = svg.indexOf(`<path aria-label="${label}" d="`);
  if (at < 0) throw new Error(`no path for ${label} in ${file}`);
  const start = at + `<path aria-label="${label}" d="`.length;
  return svg.slice(start, svg.indexOf('"', start));
}

function runs(s: string): Run[] {
  const r = parseMarkup(s, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value[0]!;
}

function width(s: string): number {
  const r = measure(s, 14, false);
  if (!r.ok) throw new Error(r.error.toString());
  return r.value.width();
}

describe("outline", () => {
  test("box_label_matches_the_minimal_render", () => {
    const x = 10 + 80 / 2 - width("A") / 2;
    const baseline = 10 + (40 - 17.5) / 2 + baselineOffset(14);
    expect(outline(runs("A"), 14, x, baseline)).toBe(
      "M45.53 35.09L49.22 24.91L50.73 24.91L54.47 35.09L53.11 35.09L52.09 32.25L47.89 32.25L46.91 35.09L45.53 35.09Z" +
        "M48.29 31.11L51.69 31.11L50.95 29.05Q50.76 28.54 50.53 27.8Q50.3 27.05 49.97 25.95Q49.65 27.07 49.41 27.83" +
        "Q49.17 28.58 49.01 29.05L48.29 31.11Z",
    );
  });

  test("accented_composite_glyphs_match_the_special_characters_render", () => {
    const label = "Accents: café naïve Ångström Straße";
    const baseline = 136 - 17.5 / 2 + baselineOffset(14);
    expect(outline(runs(label), 14, 20, baseline)).toBe(
      pathData("text/special-characters/out.svg", label),
    );
  });

  test("fallback_and_mono_lines_match_the_special_characters_render", () => {
    for (const [label, y] of [
      ["Arrows: → ← ⇒ ↔", 24],
      ["Math: ± × ÷ ≤ ≥ ≠ ≈ ∞ √ ∑ π µ °", 80],
      ["Box drawing: ┌─┬─┐ │ ├─┼─┤ └─┴─┘", 192],
    ] as const) {
      const baseline = y - 17.5 / 2 + baselineOffset(14);
      expect(outline(runs(label), 14, 20, baseline), label).toBe(
        pathData("text/special-characters/out.svg", label),
      );
    }
    const top = 420 - (3 * 17.5) / 2;
    ["┌──────┐", "│ mono │", "└──────┘"].forEach((label, i) => {
      const mono: Run[] = runs(`\`${label}\``);
      expect(mono[0]!.font).toBe("mono");
      expect(runWidth(mono[0]!, 14)).toBeCloseTo(8 * 0.6 * 14, 9);
      const baseline = top + 17.5 * i + baselineOffset(14);
      expect(outline(mono, 14, 20, baseline), label).toBe(
        pathData("text/special-characters/out.svg", label),
      );
    });
  });
});
