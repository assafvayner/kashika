import { expect, it } from "vitest";

import { toJsonString } from "../../src/diag/json.js";
import { lint } from "./common/mod.js";

it("child_outside_its_group", () => {
  const d = lint("spill", "group g at (0,0) size (200,100) {\n  box a at (150,40) size (80,40)\n}\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("box 'a' extends outside group 'g' (right by 30px)");
  expect(toJsonString(d[0]!.facts.get("overflow")!)).toBe('{"right":30.0}');
});

it("bottom_title_band_is_reserved", () => {
  const group = (y: number) =>
    `group g "VPC" at (0,0) size (200,100) title=bottom {\n  box a at (10,${y}) size (40,40)\n}\n`;
  const d = lint("spill", group(50));
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("box 'a' intrudes on the title band of group 'g'");
  expect(d[0]!.help[0]).toBe("keep children above local y=74.5");
  expect(lint("spill", group(0))).toEqual([]);
});

it("title_band_is_reserved", () => {
  const d = lint(
    "spill",
    'group g "VPC" at (0,0) size (200,100) {\n  box a at (10,10) size (40,40)\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("box 'a' intrudes on the title band of group 'g'");
  expect(
    lint("spill", 'group g "VPC" at (0,0) size (200,100) {\n  box a at (10,30) size (40,40)\n}\n'),
  ).toEqual([]);
  expect(
    lint("spill", "group g at (0,0) size (200,100) {\n  box a at (0,0) size (200,100)\n}\n"),
  ).toEqual([]);
});

it("lines_and_texts_inside_groups_spill_too", () => {
  const d = lint("spill", "group g at (0,0) size (200,100) {\n  line l (10,50) -> h 250\n}\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("line l extends outside group 'g' (right by 60px)");
  expect(
    lint("spill", 'group g at (0,0) size (200,100) {\n  text "far away" at (195,50)\n}\n').length,
  ).toBe(1);
});
