import { rmSync } from "node:fs";
import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { compile } from "../../src/model/index.js";
import { HEAD, lint, lintIn, scene, tempdir } from "./common/mod.js";

function codes(body: string): string[] {
  return check(scene(body)).map((d) => `${d.code} ${d.message}`);
}

it("layered_shapes_are_not_obstacles", () => {
  const shapes = (layer: string) =>
    `box sq at (100,100) size (100,100) fill=#eeeeee ${layer}\n` +
    `box piece "K" at (130,130) size (40,40)\n` +
    `line l (20,150) -> h 300 label="move"\n` +
    `line m (20,97) -> h 300\n` +
    `line n (150,20) -> v 77\n` +
    `text "note" at (150,190)\n`;
  const normal = codes(shapes(""));
  for (const code of ["overlap", "through", "near", "label-collision", "endpoint-off-edge"]) {
    expect(
      normal.some((d) => d.startsWith(code) && d.includes("'sq'")),
      `${code} missing: ${normal}`,
    ).toBe(true);
  }
  for (const layer of ["layer=under", "layer=over"]) {
    const d = codes(shapes(layer));
    expect(
      d.every((d) => !d.includes("'sq'") && !d.includes(" sq ")),
      `${layer}: ${d}`,
    ).toBe(true);
  }
});

it("layered_shapes_keep_their_own_checks", () => {
  let d = codes("box sq at (780,20) size (100,100) layer=under\n");
  expect(d.some((d) => d.startsWith("out-of-canvas"))).toBe(true);
  d = codes(
    'group g "G" at (0,0) size (200,200) {\n  box sq at (150,150) size (100,100) layer=over\n}\n',
  );
  expect(d.some((d) => d.startsWith("spill"))).toBe(true);
});

it("contrast_follows_paint_order", () => {
  const body = (layer: string) =>
    `box sq at (100,100) size (100,100) fill=#222222 ${layer}\ntext "dark" at (150,150) color=#333333\n`;
  expect(lint("low-contrast", body("")).length).toBe(1);
  expect(lint("low-contrast", body("layer=under")).length).toBe(1);
  expect(lint("low-contrast", body("layer=over"))).toEqual([]);
  const d = lint(
    "low-contrast",
    'box ring "R" at (100,100) size (100,100) fill=#222222 color=#333333 layer=over\n',
  );
  expect(d.length).toBe(1);
});

it("an_under_layer_fill_inside_a_group_sits_above_the_group_fill", () => {
  const body = (inner: string, outer: string) =>
    `${outer}group g at (0,0) size (300,300) fill=#1e293b {\n${inner}  text "dark" at (150,150) color=#333333\n}\n`;
  const light = "box sq at (100,100) size (100,100) fill=#fde68a layer=under\n";
  expect(lint("low-contrast", body(`  ${light}`, ""))).toEqual([]);
  expect(lint("low-contrast", body("", light)).length).toBe(1);
  expect(lint("low-contrast", body("", "")).length).toBe(1);
});

it("layer_values_and_where_it_applies", () => {
  const [, d] = compile(
    `${HEAD}box a at (0,0) size (10,10) layer=top\ngroup g at (0,0) size (10,10) layer=under {\n}\n`,
  );
  const msgs = d.map((d) => d.message);
  expect(msgs).toEqual([
    "`layer` expects one of: under, over",
    "unknown attribute `layer` on group",
  ]);
});

it("layered_images_still_report_distortion", () => {
  const dir = tempdir();
  const d = lintIn(
    dir,
    "image-distort",
    'image logo "logo.png" at (20,20) size (100,100) fit=stretch layer=under\n',
  );
  expect(d.length).toBe(1);
  rmSync(dir, { recursive: true, force: true });
});
