import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { lint, scene } from "./common/mod.js";

it("label_and_muted_sublabel", () => {
  const d = lint(
    "block-reducible",
    'box api at (20,20) size (160,60) {\n  text "Payments \\"API\\""\n  text "v2.4.1" size=11.2 muted\n}\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("the text block of box 'api' can be written as label/sublabel");
  expect(d[0]!.span.line).toBe(3);
  expect(d[0]!.help).toEqual([
    'drop the block and write `box api "Payments \\"API\\"" sublabel="v2.4.1"`',
  ]);
});

it("one_item_or_two_lines_keep_their_alignment", () => {
  let d = lint(
    "block-reducible",
    'box a at (20,20) size (160,60) text-align=left {\n  text "one"\n}\n',
  );
  expect(d[0]!.help).toEqual(['drop the block and write `box a "one" text-align=left`']);
  d = lint(
    "block-reducible",
    'box a at (20,20) size (160,60) {\n  text "one"\n  text "two"\n}\n',
  );
  expect(d[0]!.help).toEqual(['drop the block and write `box a "one\\ntwo"`']);
});

it("blocks_a_label_cannot_express", () => {
  for (const block of [
    'text "one"\n  text "two"\n  text "three"',
    'text "big" size=20',
    'text "one"\n  text "two" size=12',
    'text "one"\n  text "two" muted',
    'text "one"\n  text "two" color=#c00',
    'text "one"\n  text "two" align=left',
    'text "one" at (80,30)',
  ]) {
    const src = `box a at (20,20) size (160,90) {\n  ${block}\n}\n`;
    expect(lint("block-reducible", src), block).toEqual([]);
  }
  expect(
    lint("block-reducible", 'box a "label" at (20,20) size (160,60) sublabel="sub"\n'),
  ).toEqual([]);
});

it("allow_on_the_box_suppresses_it", () => {
  const s = scene('box a at (20,20) size (160,60) allow=block-reducible {\n  text "one"\n}\n');
  const d = check(s).filter((x) => x.code === "block-reducible");
  expect(d.length).toBe(1);
  expect(d[0]!.suppressed).toBe(true);
});
