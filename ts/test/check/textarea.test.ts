import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { lint, scene } from "./common/mod.js";

it("item_past_the_right_edge", () => {
  const d = lint(
    "text-outside-shape",
    'box e at (220,120) size (160,60) {\n  text "in" at (20,20)\n  text "out" at (150,30) align=left\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.span.line).toBe(4);
  expect(
    d[0]!.message.startsWith(
      "block item \"out\" lies outside the text area of box 'e' (right by ",
    ),
    d[0]!.message,
  ).toBe(true);
  expect(d[0]!.elements[0]!.id).toBe("e");
  expect(d[0]!.help).toEqual(["keep the item within the text area, local (8,8)..(152,52)"]);
});

it("items_inside_the_text_area", () => {
  expect(
    lint(
      "text-outside-shape",
      'box e at (220,120) size (160,60) {\n  text "in" at (80,30)\n  text "left" at (8,20) align=left\n}\n',
    ),
  ).toEqual([]);
});

it("a_diamond_text_area_is_its_center", () => {
  const d = lint(
    "text-outside-shape",
    'box d at (20,20) size (200,100) shape=diamond {\n  text "x" at (20,50)\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toContain("(left by ");
});

it("allow_on_the_box_suppresses_it", () => {
  const s = scene(
    'box e at (20,20) size (160,60) allow=text-outside-shape {\n  text "out" at (150,30)\n}\n',
  );
  const d = check(s).filter((d) => d.code === "text-outside-shape");
  expect(d.length).toBe(1);
  expect(d[0]!.suppressed).toBe(true);
});
