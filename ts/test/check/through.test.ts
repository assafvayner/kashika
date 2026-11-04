import { expect, it } from "vitest";

import { toJsonString } from "../../src/diag/json.js";
import { lint } from "./common/mod.js";

it("line_through_a_box_reports_entry_exit_and_free_lanes", () => {
  const d = lint(
    "through",
    "box a at (40,110) size (140,60)\nbox cache at (300,110) size (80,60)\nbox c at (520,110) size (100,60)\nline l a.right -> c.left\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l passes through box 'cache'");
  expect(toJsonString(d[0]!.facts.get("entry")!)).toBe("[300.0,140.0]");
  expect(toJsonString(d[0]!.facts.get("exit")!)).toBe("[380.0,140.0]");
  expect(d[0]!.elements[1]!.bbox).toEqual([
    [300, 110],
    [380, 170],
  ]);
  expect(d[0]!.help).toEqual([
    "segment 1 runs at y=140; free y-intervals across x 180..520: 0..104, 176..600",
    "if 'cache' is a stop on this path, split the line at it: one line ending at 'cache'.left, another starting at 'cache'.right",
  ]);
});

it("a_line_through_the_middle_of_a_box_suggests_splitting_at_it", () => {
  const help = (body: string): string | null => {
    const d = lint("through", body);
    expect(d.length).toBe(1);
    return d[0]!.help.find((h) => h.includes("is a stop on this path")) ?? null;
  };
  expect(help("box m at (100,100) size (80,60)\nline l (140,300) -> v -280\n")).toBe(
    "if 'm' is a stop on this path, split the line at it: one line ending at 'm'.bottom, another starting at 'm'.top",
  );
  expect(help("box m at (100,100) size (80,60)\nline l (300,130) -> h -280\n")).toBe(
    "if 'm' is a stop on this path, split the line at it: one line ending at 'm'.right, another starting at 'm'.left",
  );
  expect(help("box m at (100,100) size (80,60)\nline l (0,105) -> h 300\n")).toBeNull();
  expect(
    help("box m at (100,100) size (80,60)\nline l (0,130) -> (140,130) -> v 200\n"),
  ).toBeNull();
});

it("own_boxes_are_exempt_only_on_the_first_and_last_segments", () => {
  expect(
    lint(
      "through",
      "box a at (0,0) size (100,100)\nbox b at (300,0) size (100,100)\nline l a.center -> b.center\n",
    ),
  ).toEqual([]);
  const d = lint(
    "through",
    "box a at (0,0) size (100,100)\nline l a.right -> h 50 -> v 150 -> x 50 -> y 50 -> h -100\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.elements[0]!.segment).toBe(4);
});

it("groups_are_never_obstacles", () => {
  expect(
    lint("through", "group g at (0,0) size (300,300) {\n}\nline l (10,150) -> h 200\n"),
  ).toEqual([]);
});

it("reaching_an_edge_anchor_from_inside_its_own_box_is_flagged", () => {
  let d = lint(
    "through",
    "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100)\nline l a.right -> (240,45) -> vh b.left\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs through the inside of its own endpoint box 'b'");
  expect(d[0]!.elements[1]!.id).toBe("b");
  expect(toJsonString(d[0]!.facts.get("entry")!)).toBe("[200.0,45.0]");
  expect(toJsonString(d[0]!.facts.get("exit")!)).toBe("[200.0,70.0]");
  expect(d[0]!.help[0]!.startsWith("arrive at 'b' from outside")).toBe(true);

  d = lint(
    "through",
    "box a at (20,20) size (100,100)\nbox b at (300,20) size (60,50)\nline l a.right -> h -40 -> v 150 -> hv b.bottom\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs through the inside of its own endpoint box 'a'");
  expect(toJsonString(d[0]!.facts.get("entry")!)).toBe("[120.0,70.0]");
  expect(toJsonString(d[0]!.facts.get("exit")!)).toBe("[80.0,120.0]");
  expect(d[0]!.help[0]!.startsWith("leave 'a' straight out")).toBe(true);
});

it("normal_attachments_to_own_boxes_are_not_flagged", () => {
  const boxes = "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100)\n";
  for (const line of [
    "line l a.right -> (150,45) -> vh b.left end=arrow start=diamond",
    "line l a.top -> v -10 -> x 250 -> b.top",
    "line l a.bottom -> v 60 -> x 220 -> y 120 -> b.bottom",
    "line l a.center -> b.center",
    "line l a.center+(10,5) -> hv b.left",
  ]) {
    const src = `${boxes}${line}\n`;
    expect(lint("through", src), src).toEqual([]);
  }
  for (const src of [
    'box a at (20,20) size (60,50) icon="lock"\nshape srv from "server" text-area=below\nbox b "B" at (200,20) size (100,60) shape=srv\nline l a.right -> b.left end=arrow\nline m a.bottom -> v 60 -> hv b.bottom\n',
    "box a at (20,20) size (60,50)\nbox b at (200,20) size (100,100) layer=under\nline l a.right -> (240,45) -> vh b.left\n",
  ]) {
    expect(lint("through", src), src).toEqual([]);
  }
});
