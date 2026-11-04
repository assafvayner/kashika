import { describe, expect, test } from "vitest";

import { BUNDLED_ICONS, bundledIcon, ICON_KEYWORDS } from "../../src/model/icons.js";
import { SHAPES } from "../../src/model/shapes.js";
import { STYLES } from "../../src/model/style.js";

function unique(names: Iterable<string>): boolean {
  const seen = new Set<string>();
  for (const n of names) {
    if (seen.has(n)) return false;
    seen.add(n);
  }
  return true;
}

describe("registry", () => {
  test("every_builtin_shape_has_a_unique_name_and_keywords", () => {
    expect(unique(SHAPES.map((s) => s.name))).toBe(true);
    for (const s of SHAPES) {
      expect(s.keywords.length > 0, `shape \`${s.name}\` has no keywords`).toBe(true);
    }
  });

  test("every_bundled_icon_has_keywords_and_every_keywords_entry_an_icon", () => {
    expect(unique(BUNDLED_ICONS)).toBe(true);
    expect(unique(ICON_KEYWORDS.map(([n]) => n))).toBe(true);
    for (const name of BUNDLED_ICONS) {
      const words = ICON_KEYWORDS.find(([n]) => n === name);
      expect(
        words !== undefined && words[1].length > 0,
        `icons/${name}.svg has no line in icons/keywords.toml`,
      ).toBe(true);
    }
    for (const [name] of ICON_KEYWORDS) {
      expect(
        BUNDLED_ICONS.includes(name),
        `icons/keywords.toml names \`${name}\`, but there is no icons/${name}.svg`,
      ).toBe(true);
    }
  });

  test("bundled_icons_keep_the_keywords_file_order", () => {
    expect([...BUNDLED_ICONS]).toEqual(ICON_KEYWORDS.map(([n]) => n));
  });

  test("every_builtin_style_has_a_unique_name", () => {
    expect(unique(STYLES.map((s) => s.name))).toBe(true);
  });

  test("bundled_icon_entries_are_typed", () => {
    const s = bundledIcon("server");
    expect(s).not.toBeNull();
    expect(s!.natural).toEqual([24, 24]);
    expect(s!.svg).toContain("#010203");
    expect(s!.source.startsWith("<svg")).toBe(true);
    expect(bundledIcon("nope")).toBeNull();
    expect(bundledIcon("constructor")).toBeNull();
  });
});
