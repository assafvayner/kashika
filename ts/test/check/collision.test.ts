import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("label_over_a_box_suggests_clear_positions", () => {
  const d = lint(
    "label-collision",
    'box b at (180,80) size (40,40)\nline l (0,100) -> h 400 label="SQL"\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("label \"SQL\" of l collides with box 'b'");
  expect(d[0]!.help).toEqual(["clear label positions on l: label@0.05..0.4, label@0.6..0.95"]);
});

it("texts_lines_and_other_labels", () => {
  expect(
    lint("label-collision", 'line a (0,100) -> h 400\ntext "note" at (200,100)\n').length,
  ).toBe(1);
  expect(
    lint("label-collision", 'text "one" at (100,100)\ntext "two" at (105,102)\n').length,
  ).toBe(1);
  expect(
    lint("label-collision", 'line a (0,100) -> h 400 label="x"\ntext "note" at (200,50)\n'),
  ).toEqual([]);
});

it("group_titles_are_obstacles", () => {
  const d = lint(
    "label-collision",
    'group g "VPC" at (0,0) size (300,200) {\n}\ntext "x" at (20,17)\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("text 'text@L4' collides with the title of group 'g'");
});

it("label_over_its_own_arrowhead", () => {
  let d = lint("label-collision", 'line l (20,100) -> h 40 end=arrow label="hello"\n');
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe('label "hello" of l collides with the end arrowhead of l');
  expect(d[0]!.elements[1]!.id).toBe("l");
  expect(d[0]!.help).toEqual(["clear label positions on l: label@0.05..0.4"]);
  d = lint("label-collision", 'line l (20,100) -> h 40 start=diamond label@0.2="hello"\n');
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe('label "hello" of l collides with the start diamond of l');
});

it("label_beside_its_line_clears_the_arrowhead", () => {
  expect(
    lint(
      "label-collision",
      'line l (20,100) -> h 40 end=arrow label="hello" label-side=above\n',
    ),
  ).toEqual([]);
});

it("label_beside_its_line_collides_with_its_other_segments", () => {
  const d = lint(
    "label-collision",
    'line l (20,100) -> h 100 -> v -100 label@0.45="hello" label-side=above\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe('label "hello" of l collides with line l');
  expect(d[0]!.help).toEqual(["clear label positions on l: label@0.05..0.4"]);
});

it("label_on_its_own_stroke_away_from_markers", () => {
  expect(
    lint(
      "label-collision",
      'line l (20,100) -> h 300 -> v 100 start=dot end=arrow label="hello" label@0.9="x"\n',
    ),
  ).toEqual([]);
});

it("markers_on_thick_lines_reach_further", () => {
  const line = (w: string, end: string) =>
    `line l (20,100) -> h 300 end=${end} stroke-width=${w} label@0.9="x"\n`;
  expect(lint("label-collision", line("1.5", "arrow"))).toEqual([]);
  for (const [end, name] of [
    ["arrow", "arrowhead"],
    ["triangle", "triangle"],
    ["circle", "circle"],
  ] as const) {
    const d = lint("label-collision", line("6", end));
    expect(d.length).toBe(1);
    expect(d[0]!.message).toBe(`label "x" of l collides with the end ${name} of l`);
  }
});

it("label_over_the_boxes_its_line_connects", () => {
  const d = lint(
    "label-collision",
    'box a at (20,80) size (80,40)\nbox b at (140,80) size (80,40)\nline l a.right -> b.left label="calls the api"\n',
  );
  const hits = d.map((x) => x.message);
  expect(hits).toEqual([
    "label \"calls the api\" of l collides with box 'a'",
    "label \"calls the api\" of l collides with box 'b'",
  ]);
  expect(
    d[0]!.help[0]!.startsWith(
      "no clear label position on l: the label is 68px wide and its segment 40px long; give it about 74px clear",
    ),
  ).toBe(true);
});

it("a_label_too_wide_for_its_run_suggests_a_side", () => {
  const d = lint(
    "label-collision",
    'line l (20,100) -> h 40 start=arrow end=arrow label="hello"\n',
  );
  expect(d.length).toBe(2);
  expect(d[0]!.help[0]!.endsWith("or put it beside the line with `label-side=above`")).toBe(true);
});

it("free_text_over_a_box_suggests_its_block", () => {
  let d = lint(
    "label-collision",
    'box a at (20,20) size (200,80)\ntext "note" at (120,70)\n',
  );
  expect(d.length).toBe(1);
  expect(d[0]!.help).toEqual([
    "to put the text inside 'a', move it into its text block, in box-local coordinates: `box a … { text \"note\" at (100,50) }`",
  ]);
  d = lint("label-collision", 'box a at (20,20) size (200,80)\ntext "note" at (120,105)\n');
  expect(d[0]!.help).toEqual([]);
});

it("labels_on_group_borders", () => {
  const groups =
    'group a "A" at (0,0) size (200,200) {\n}\ngroup b "B" at (230,0) size (200,200) {\n}\n';
  const d = lint(
    "label-collision",
    `${groups}line l (100,100) -> h 230 label="HTTPS"\n`,
  );
  expect(d.length).toBe(2);
  expect(d[0]!.message).toBe('label "HTTPS" of l collides with the border of group \'a\'');
  expect(d[1]!.message).toBe('label "HTTPS" of l collides with the border of group \'b\'');
  expect(d[0]!.help).toEqual([
    "clear label positions on l: label@0.05..0.35, label@0.65..0.95",
  ]);
  expect(
    lint("label-collision", `${groups}line l (100,100) -> h 230 label@0.2="HTTPS"\n`),
  ).toEqual([]);
  expect(
    lint("label-collision", `${groups}text "inside" at (100,100)\ntext "x" at (215,150)\n`),
  ).toEqual([]);
});

it("lines_crossing_group_titles", () => {
  const group =
    'group pub "public subnet" at (0,0) size (300,200) {\n  box nat "NAT" at (20,60) size (80,40)\n}\n';
  let d = lint("label-collision", `${group}line l nat.top -> y -40\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l crosses the title of group 'pub'");
  expect(d[0]!.elements[1]!.id).toBe("pub");
  expect(d[0]!.help[0]!.startsWith("the title of 'pub' covers x 8..")).toBe(true);
  expect(d[0]!.help[1]).toBe("or move the title away from the line: `text-align=right` on 'pub'");
  expect(d[0]!.help[2]).toBe("or move the title to the bottom edge: `title=bottom` on 'pub'");
  d = lint("label-collision", `${group}line l (40,-20) -> v 240\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.help.length).toBe(2);
  const bottom = group.replace("size (300,200)", "size (300,200) title=bottom");
  expect(lint("label-collision", `${bottom}line l nat.top -> y -40\n`)).toEqual([]);
  d = lint("label-collision", `${bottom}line l nat.bottom -> y 240\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.help[0]!.endsWith("inside the group above y=174.5 or outside it")).toBe(true);
  expect(d[0]!.help[2]).toBe("or move the title to the top edge: `title=top` on 'pub'");
  expect(
    lint(
      "label-collision",
      `${group.replace("size (300,200)", "size (300,200) text-align=right")}line l nat.top -> y -40\n`,
    ),
  ).toEqual([]);
  expect(
    lint("label-collision", `${group}line l nat.right -> h 100 -> v -100\n`),
  ).toEqual([]);
});

it("label_over_its_own_crows_foot", () => {
  let d = lint("label-collision", 'line l (20,100) -> h 60 start=zero-many label@0.15="x"\n');
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe('label "x" of l collides with the start crow\'s foot of l');
  d = lint("label-collision", 'line l (20,100) -> h 60 end=one label@0.85="x"\n');
  expect(d.map((x) => x.message)).toEqual(['label "x" of l collides with the end bar of l']);
  expect(
    lint(
      "label-collision",
      'line l (20,100) -> h 200 start=zero-many end=one-many label="x"\n',
    ),
  ).toEqual([]);
});
