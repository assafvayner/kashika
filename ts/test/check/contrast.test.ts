import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { toJsonString } from "../../src/diag/json.js";
import { lint, lintIn, sceneSrc, tempdir } from "./common/mod.js";

it("light_text_on_a_light_fill", () => {
  const d = lint(
    "low-contrast",
    'box a "API" at (0,0) size (100,60) fill=#ffffff color=#dddddd\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe(
    "label \"API\" of 'a' has contrast 1.4:1 against its background (minimum 3:1)",
  );
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#ffffff"');
  expect(lint("low-contrast", 'box a "API" at (0,0) size (100,60) style=service\n')).toEqual([]);
  expect(
    lint("low-contrast", "box a at (0,0) size (100,60) fill=#ffffff color=#ffffff\n"),
  ).toEqual([]);
});

it("text_takes_the_fill_of_the_shapes_under_it", () => {
  expect(
    lint(
      "low-contrast",
      'box a at (0,0) size (200,100) fill=#000000\ntext "note" at (100,50) color=#111111\n',
    ).length,
  ).toBe(1);
  expect(
    lint(
      "low-contrast",
      'box a at (0,0) size (200,100) fill=#000000\ntext "note" at (100,50) color=#ffffff\n',
    ),
  ).toEqual([]);
});

it("aligned_text_takes_the_fill_under_its_center_not_its_anchor", () => {
  const dark = "box a at (104,0) size (200,100) fill=#000000\n";
  expect(
    lint(
      "low-contrast",
      `${dark}text "a long note" at (100,50) align=left color=#ffffff\n`,
    ),
  ).toEqual([]);
  const d = lint(
    "low-contrast",
    `${dark}text "a long note" at (100,50) align=left color=#111111\n`,
  );
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#000000"');
  expect(
    lint(
      "low-contrast",
      `${dark}text "a long note" at (308,50) align=right color=#ffffff\n`,
    ),
  ).toEqual([]);
});

it("emoji_keep_their_own_colors_and_are_not_judged", () => {
  const src = (label: string) =>
    `box a "${label}" at (0,0) size (100,60) fill=#ffffff color=#eeeeee\n` +
    `text "${label}" at (200,50) color=#eeeeee\n` +
    "style faint { color=#eeeeee }\n" +
    `line l (0,120) -> h 100 label="${label}" style=faint\n`;
  expect(lint("low-contrast", src("🚀 ✅"))).toEqual([]);
  expect(lint("low-contrast", src("🚀 ok")).length).toBe(3);
});

it("line_labels_on_a_bare_canvas_count_as_white", () => {
  expect(
    lint(
      "low-contrast",
      'style faint { color=#eeeeee }\nline l (0,0) -> h 100 label="x" style=faint\n',
    ).length,
  ).toBe(1);
});

it("sublabels_use_the_muted_color", () => {
  const d = lint(
    "low-contrast",
    'box a "API" at (0,0) size (100,60) sublabel="v2" fill=#64748b\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message.startsWith("sublabel \"v2\" of 'a'")).toBe(true);
});

it("canvas_text_is_checked_against_the_background", () => {
  const on = (bg: string, color: string) => {
    const src = `diagram "t" size (400,200)${bg}\ntext "note" at (100,50) color=${color}\n`;
    return check(sceneSrc(src)).filter((d) => d.code === "low-contrast");
  };
  expect(on("", "#eeeeee").length).toBe(1);
  let d = on(" background=#ffffff", "#eeeeee");
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#ffffff"');
  expect(on(" background=#0f172a", "#eeeeee")).toEqual([]);
  d = on(" background=#0f172a", "#111111");
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#0f172a"');
  expect(on(" background=none", "#111111")).toEqual([]);
});

it("line_labels_take_the_fill_beneath_them", () => {
  const src = (fill: string) =>
    `style pale { color=#f8fafc }\ngroup g at (0,0) size (300,100) fill=${fill} {\n  line l (20,50) -> (280,50) label="x" style=pale\n}\n`;
  expect(lint("low-contrast", src("#0f172a"))).toEqual([]);
  expect(lint("low-contrast", src("#ffffff")).length).toBe(1);
  const s =
    'diagram "t" size (400,200) background=#0f172a\nstyle pale { color=#f8fafc }\nline l (0,50) -> h 100 label="x" style=pale\n';
  expect(check(sceneSrc(s)).every((d) => d.code !== "low-contrast")).toBe(true);
});

it("named_colors_are_checked_like_hex", () => {
  let d = lint("low-contrast", 'text "note" at (100,50) color=lightgray\n');
  expect(d.length).toBe(1);
  const src =
    'diagram "t" size (400,200) background=white\ntext "note" at (100,50) color=lightgray\n';
  d = check(sceneSrc(src)).filter((x) => x.code === "low-contrast");
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#ffffff"');
});

it("icons_drawn_in_the_shape_color_are_checked", () => {
  const d = lint("low-contrast", 'box k at (0,0) size (100,60) fill=#1e293b icon="lock"\n');
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("icon of 'k' has contrast 1.4:1 against its background (minimum 3:1)");
  expect(toJsonString(d[0]!.facts.get("color")!)).toBe('"#334155"');
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#1e293b"');
  expect(d[0]!.help[0]!.includes("`color`")).toBe(true);
  expect(
    lint(
      "low-contrast",
      'box k "Vault" at (0,0) size (100,60) fill=#1e293b color=white icon="lock"\n',
    ),
  ).toEqual([]);
});

it("user_icons_are_checked_only_when_they_use_current_color", () => {
  const dir = tempdir();
  try {
    const svg = (paint: string) =>
      `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect x="4" y="4" width="16" height="16" fill="${paint}"/></svg>`;
    writeFileSync(join(dir, "cur.svg"), svg("currentColor"));
    writeFileSync(join(dir, "fixed.svg"), svg("#1e293b"));
    const d = lintIn(
      dir,
      "low-contrast",
      'box a at (0,0) size (100,60) fill=#1e293b icon="cur.svg"\n' +
        'box b at (0,100) size (100,60) fill=#1e293b icon="fixed.svg"\n' +
        'box c at (0,200) size (100,60) fill=#1e293b color=#f8fafc icon="cur.svg"\n',
    );
    expect(d.length).toBe(1);
    expect(d[0]!.elements[0]!.id).toBe("a");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
