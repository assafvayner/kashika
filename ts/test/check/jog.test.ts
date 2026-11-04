import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { toJsonString } from "../../src/diag/json.js";
import { lint, sceneSrc } from "./common/mod.js";

it("short_step_between_vertical_runs_suggests_the_level_anchors", () => {
  const d = lint(
    "jog",
    "box a at (480,40) size (100,60)\nbox b at (505,300) size (60,60)\nline l a.bottom -> v 20 -> hv b.top@0.35\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe(
    "segment 2 of l jogs 4px in x between two vertical runs, from (530,120) to (526,120)",
  );
  expect(d[0]!.help).toEqual([
    "end at `b.top@0.42` to keep x=530",
    "start at `a.bottom@0.46` to keep x=526",
    "or move the run after the step to x=530 or the run before it to x=526, so both vertical runs share x",
  ]);
  expect(d[0]!.elements[0]!.segment).toBe(2);
  expect(toJsonString(d[0]!.facts.get("step")!)).toBe("4.0");
});

it("mid_path_step_between_horizontal_runs", () => {
  const d = lint("jog", "line l (20,100) -> h 100 -> v -3 -> h 100 -> v 200\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe(
    "segment 2 of l jogs 3px in y between two horizontal runs, from (120,100) to (120,97)",
  );
  expect(d[0]!.help).toEqual([
    "move the run after the step to y=100 or the run before it to y=97, so both horizontal runs share y",
  ]);
});

it("rounded_and_ortho_lines_still_jog", () => {
  for (const attrs of ["radius=8", "ortho"]) {
    const body = `line l (20,100) -> h 100 -> v 4 -> h 100 ${attrs}\n`;
    expect(lint("jog", body).length, attrs).toBe(1);
  }
});

it("u_turns_long_steps_curves_and_straight_runs_are_fine", () => {
  for (const body of [
    "line l (20,100) -> h 100 -> v 4 -> h -100\n",
    "line l (20,100) -> h 100 -> v 6 -> h 100\n",
    "line l (20,100) -> h 100 -> curve left 2 (120,104) -> h 100\n",
    "line l (20,100) -> h 100 -> v 0 -> h 100\n",
    "line l (20,100) -> h 100 -> h 100 -> v 200\n",
    "line l (20,100) -> v 4 -> h 100\n",
  ]) {
    expect(lint("jog", body), body).toEqual([]);
  }
});

it("threshold_follows_clearance", () => {
  const src =
    'diagram "t" size (800,600) clearance=12\nline l (20,100) -> h 100 -> v 10 -> h 100\n';
  const d = check(sceneSrc(src)).filter((x) => x.code === "jog");
  expect(d.length).toBe(1);
});

it("allow_suppresses", () => {
  const d = lint("jog", 'line l (20,100) -> h 100 -> v 4 -> h 100 allow=jog reason="bus offset"\n');
  expect(d.length).toBe(1);
  expect(d[0]!.suppressed).toBe(true);
});
