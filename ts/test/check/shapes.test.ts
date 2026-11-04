import { rmSync } from "node:fs";
import { expect, it } from "vitest";

import { Ctx, check } from "../../src/check/index.js";
import { ports } from "../../src/check/ports.js";
import { toJsonString } from "../../src/diag/json.js";
import { lint, lintIn, scene, tempdir } from "./common/mod.js";

const DIAMOND = "box d at (100,100) size (100,100) shape=diamond\n";
const CYLINDER = "box db at (100,100) size (100,100) shape=cylinder\n";

it("lines_through_a_diamond_follow_its_outline", () => {
  const d = lint("through", `${DIAMOND}line l (0,150) -> (300,150)\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l passes through box 'd'");
  expect(toJsonString(d[0]!.facts.get("entry")!)).toBe("[100.0,150.0]");
  expect(lint("through", `${DIAMOND}line l (90,110) -> (110,90)\n`)).toEqual([]);
});

it("near_measures_to_the_diamond_edge", () => {
  const d = lint("near", `${DIAMOND}line l (100,145) -> (145,100)\n`);
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("distance")!)).toBe("3.54");
  expect(lint("near", `${DIAMOND}line l (90,140) -> (140,90)\n`)).toEqual([]);
});

it("diamonds_overlap_only_when_their_outlines_do", () => {
  const a = "box a at (0,0) size (100,100) shape=diamond\n";
  expect(lint("overlap", `${a}box b at (80,80) size (100,100) shape=diamond\n`)).toEqual([]);
  const d = lint("overlap", `${a}box b at (40,40) size (100,100) shape=diamond\n`);
  expect(d.length).toBe(1);
});

it("endpoints_snap_to_the_cylinder_cap", () => {
  const d = lint("endpoint-off-edge", `${CYLINDER}line l (150,40) -> (150,97)\n`);
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("nearest")!)).toBe("[150.0,100.0]");
  expect(d[0]!.help).toEqual(["snap it to the outline with an anchor: `db.top`"]);
  expect(lint("endpoint-off-edge", `${CYLINDER}line l (110,40) -> (110,108)\n`)).toEqual([]);
});

it("anchored_lines_on_built_in_shapes_check_clean", () => {
  const s = scene(
    `${CYLINDER}box d at (300,100) size (100,100) shape=diamond\n` +
      `line a d.left -> db.right end=arrow\n` +
      `line b db.top@0.2 -> v -60 end=arrow\n` +
      `line c d.top@0.3 -> v -60 end=arrow\n`,
  );
  expect(check(s)).toEqual([]);
  const ctx = Ctx.new(s);
  const got = ports(ctx).map((p) => `${p.shape}.${p.side}@${Math.round(p.t * 100) / 100}`);
  expect(got).toEqual(["db.right@0.5", "db.top@0.2", "d.left@0.5", "d.top@0.3"]);
});

const LOGO = 'image logo "logo.png" at (100,100) size (100,50)\n';

it("images_are_opaque_obstacles", () => {
  const dir = tempdir();
  let d = lintIn(dir, "through", `${LOGO}line l (0,125) -> (300,125)\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l passes through image 'logo'");
  expect(d[0]!.elements[1]!.kind).toBe("image");
  expect(d[0]!.help[0]).toContain("0..94, 156..600");
  d = lintIn(dir, "near", `${LOGO}line l (0,97) -> (300,97)\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs 3px from image 'logo' (clearance 6)");
  d = lintIn(dir, "overlap", `${LOGO}box b at (180,120) size (60,60)\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("box 'b' overlaps image 'logo'");
  d = lintIn(dir, "label-collision", `${LOGO}line l (0,125) -> (300,125) label="to logo"\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message.endsWith("collides with image 'logo'")).toBe(true);
  d = lintIn(dir, "out-of-canvas", 'image logo "logo.png" at (760,0) size (80,40)\n');
  expect(d.length).toBe(1);
  expect(d[0]!.message.startsWith("image 'logo' extends beyond")).toBe(true);
  rmSync(dir, { recursive: true, force: true });
});

it("lines_may_attach_to_images", () => {
  const dir = tempdir();
  for (const code of ["through", "near", "endpoint-off-edge"]) {
    expect(
      lintIn(
        dir,
        code,
        `${LOGO}box b at (400,100) size (100,50)\nline l logo.right -> b.left\n`,
      ),
    ).toEqual([]);
  }
  rmSync(dir, { recursive: true, force: true });
});

const BELOW = 'shape srv from "server" text-area=below\n';

it("labels_below_a_shape_are_checked_where_they_sit", () => {
  let d = lint(
    "out-of-canvas",
    `${BELOW}box s "Uploads" at (100,530) size (60,60) shape=srv\n`,
  );
  expect(d.length).toBe(1);
  expect(
    d[0]!.message.startsWith("the label under box 's' extends beyond the 800x600 canvas"),
    d[0]!.message,
  ).toBe(true);
  const body = `${BELOW}box s "Uploads" at (100,100) size (60,60) shape=srv\n`;
  d = lint("label-collision", `${body}line l (0,177) -> (300,177)\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("the label under box 's' collides with line l");
  expect(d[0]!.help).toEqual([]);
  expect(lint("label-collision", body)).toEqual([]);
  d = lint(
    "spill",
    `${BELOW}group g at (80,80) size (120,100) {\n  box s "Uploads" at (20,20) size (60,60) shape=srv\n}\n`,
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message.startsWith("the label under box 's' extends outside group 'g'")).toBe(true);
});

it("a_line_leaving_through_the_label_below_suggests_another_side", () => {
  const body = `${BELOW}box s "Uploads" at (100,100) size (60,60) shape=srv\n`;
  const d = lint("label-collision", `${body}line l s.bottom -> v 100\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.help).toEqual([
    "move the line to another side of 's', or give 's' an `icon=` with the text inside the box instead of below",
  ]);
});

it("labels_below_a_shape_sit_on_the_canvas_not_the_fill", () => {
  const d = lint(
    "low-contrast",
    `${BELOW}box s "Uploads" at (100,100) size (60,60) shape=srv fill=#000000 color=#ffffff\n`,
  );
  expect(d.length).toBe(1);
  expect(toJsonString(d[0]!.facts.get("background")!)).toBe('"#ffffff"');
  expect(
    lint(
      "low-contrast",
      'box s "Uploads" at (100,100) size (120,60) fill=#000000 color=#ffffff\n',
    ),
  ).toEqual([]);
});

it("icons_shrink_the_text_area_without_warnings", () => {
  const s = scene(
    'box q "Jobs" at (100,100) size (160,60) icon="queue" style=queue\n' +
      'box w "Web" at (100,200) size (160,80) icon="browser" icon-pos=top\n' +
      "line l q.bottom -> w.top end=arrow\n",
  );
  expect(check(s)).toEqual([]);
  expect(s.shapes[0]!.textArea.x >= 100 + 24).toBe(true);
});
