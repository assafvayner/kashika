import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { compile } from "../../src/model/index.js";
import { HEAD, lint } from "./common/mod.js";

it("slight_slant_suggests_the_level_anchor", () => {
  const d = lint(
    "near-axis",
    "box users at (20,100) size (100,60)\nbox cdn at (400,105) size (100,60)\nline l users.right -> cdn.left\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe(
    "segment 1 of l is 5px (1.02°) off horizontal, from (120,130) to (400,135)",
  );
  expect(d[0]!.help).toEqual([
    "end at `cdn.left@0.42` to keep y=130",
    "start at `users.right@0.58` to keep y=135",
    "or move one end 5px in y so both ends share it, or use `hv`/`vh` (or an `x`/`y` target) for a deliberate elbow",
  ]);
});

it("vertical_slant_and_offset_anchor", () => {
  const d = lint(
    "near-axis",
    "box a at (100,20) size (90,40)\nbox b at (103,300) size (90,40)\nline l a.bottom -> b.top\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toContain("3px");
  expect(d[0]!.message).toContain("off vertical");
  expect(d[0]!.help[0]).toBe("end at `b.top+(-3,0)` to keep x=145");
});

it("axis_aligned_steep_ortho_and_curves_are_fine", () => {
  for (const body of [
    "line l (0,100) -> h 300\n",
    "line l (0,100) -> (300,100.4)\n",
    "line l (0,100) -> (300,130)\n",
    "line l (0,100) -> (300,105) ortho\n",
    "line l (0,100) -> curve left 2 (300,105)\n",
  ]) {
    const [s] = compile(`${HEAD}${body}`);
    const d = check(s).filter((d) => d.code === "near-axis");
    expect(d, body).toEqual([]);
  }
});

it("near_axis_tails_are_not_diagonal_tails", () => {
  const body = "line l (0,0) -> h 100 -> v 50 -> (300,55)\n";
  expect(lint("near-axis", body).length).toBe(1);
  expect(lint("diagonal-tail", body)).toEqual([]);
});

it("offset_anchor_when_t_does_not_land_exactly", () => {
  const d = lint(
    "near-axis",
    "box a at (20,100) size (100,600)\nbox b at (400,0) size (100,810)\nline l a.right -> b.left\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.help[0]).toBe("end at `b.left+(0,-5)` to keep y=400");
});
