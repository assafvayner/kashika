import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { lint, sceneSrc } from "./common/mod.js";

it("shapes_and_lines_beyond_the_canvas", () => {
  const d = lint("out-of-canvas", "box a at (750,10) size (100,60)\nline l (10,10) -> v 700\n");
  expect(d.length).toBe(2);
  expect(d[0]!.message).toBe("box 'a' extends beyond the 800x600 canvas (right by 50px)");
  expect(d[1]!.message).toBe("line l extends beyond the 800x600 canvas (bottom by 110px)");
  expect(lint("out-of-canvas", "box a at (700,540) size (100,60)\n")).toEqual([]);
});

it("content_above_or_left_of_the_origin_must_move", () => {
  let d = lint("out-of-canvas", "box a at (750,10) size (100,60)\n");
  expect(d[0]!.help).toEqual([
    "move it inside (0,0)-(800,600) or enlarge `size` on the `diagram` line",
  ]);
  const s = sceneSrc('diagram "t"\nbox a at (-30,20) size (100,40)\n');
  d = check(s).filter((x) => x.code === "out-of-canvas");
  expect(d.length).toBe(1);
  expect(d[0]!.help.length).toBe(1);
  expect(d[0]!.help[0]!.startsWith("move it right by 30px")).toBe(true);
  expect(d[0]!.help[0]!.endsWith("a fitted canvas grows only right and down")).toBe(true);
});

it("children_of_an_escaping_group_are_not_reported_again", () => {
  let d = lint(
    "out-of-canvas",
    'group g "G" at (700,10) size (200,100) {\n  box a at (10,30) size (180,40)\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message.startsWith("group 'g'")).toBe(true);
  d = lint(
    "out-of-canvas",
    'group g "G" at (100,10) size (200,100) {\n  box a at (750,30) size (80,40)\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message.startsWith("box 'a'")).toBe(true);
});
