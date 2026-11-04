import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { toJsonString } from "../../src/diag/json.js";
import { lint, sceneSrc } from "./common/mod.js";

it("line_passing_close_to_a_box", () => {
  const d = lint("near", "box a at (100,100) size (100,60)\nline l (0,164) -> h 300\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs 4px from box 'a' (clearance 6)");
  expect(toJsonString(d[0]!.facts.get("distance")!)).toBe("4.0");
});

it("running_along_its_own_box_edge", () => {
  let d = lint("near", "box a at (100,100) size (100,60)\nline l a.tl+(10,0) -> h 80 -> v -50\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l's first segment runs along the top side of its own box 'a'");
  d = lint(
    "near",
    "box a at (100,100) size (100,60)\nline l a.top -> v -20 -> h 50 -> v 60 -> h 40\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs along the edge of its own box 'a', 0px away (clearance 6)");
  expect(
    lint(
      "near",
      "box a at (100,100) size (100,60)\nbox b at (400,100) size (100,60)\nline l a.right -> b.left\n",
    ),
  ).toEqual([]);
});

it("a_wrong_elbow_into_its_own_box_names_the_fix", () => {
  let d = lint(
    "near",
    "box a at (20,20) size (100,40)\nbox b at (220,100) size (100,40)\nline l a.right -> hv b.left\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l's last segment runs along the left side of its own box 'b'");
  expect(d[0]!.help[0]).toContain("finish with `vh`");
  d = lint(
    "near",
    "box a at (20,140) size (100,40)\nbox b at (220,20) size (100,40)\nline l a.right -> vh b.bottom\n",
  );
  expect(d.length).toBe(2);
  expect(d[0]!.message).toBe("l's first segment runs along the right side of its own box 'a'");
  expect(d[0]!.help[0]).toContain("start with `hv`");
  expect(d[1]!.help[0]).toContain("finish with `hv`");
  expect(
    lint(
      "near",
      "box a at (20,20) size (100,40)\nbox b at (220,100) size (100,40)\nline l a.right -> hvh b.left\n",
    ),
  ).toEqual([]);
});

it("a_line_through_a_box_is_not_also_near_it", () => {
  expect(lint("near", "box a at (100,100) size (100,60)\nline l (0,130) -> h 300\n")).toEqual([]);
});

it("parallel_lines_closer_than_clearance_for_over_20px", () => {
  const d = lint("near", "line a (0,100) -> h 300\nline b (0,104) -> h 300\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("b runs 4px from a for 300px (clearance 6)");
  expect(lint("near", "line a (0,100) -> h 300\nline b (0,104) -> h 15\n")).toEqual([]);
  expect(lint("near", "line a (0,100) -> h 300\nline b (0,110) -> h 300\n")).toEqual([]);
});

it("clearance_comes_from_the_diagram", () => {
  const s = sceneSrc(
    'diagram "t" size (800,600) clearance=12\nline a (0,100) -> h 300\nline b (0,110) -> h 300\n',
  );
  expect(check(s).filter((d) => d.code === "near").length).toBe(1);
});

it("running_along_a_group_border", () => {
  const g = 'group vpc "VPC" at (200,20) size (300,300) {\n}\n';
  let d = lint("near", `${g}line l (100,100) -> x 200 -> v 100\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs along the border of group 'vpc' (clearance 6)");
  expect(d[0]!.elements[0]!.segment).toBe(2);
  expect(d[0]!.help).toEqual([
    "segment 2 runs at x=200; free x-intervals across y 100..200: 0..194, 206..494, 506..800",
  ]);
  d = lint("near", `${g}line l (100,316) -> h 300\n`);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l runs 4px from the border of group 'vpc' (clearance 6)");
});

it("crossing_a_group_border_is_not_near_it", () => {
  const g = 'group vpc "VPC" at (200,20) size (300,300) {\n}\n';
  expect(lint("near", `${g}line l (100,200) -> h 200 -> v 50\n`)).toEqual([]);
  expect(lint("near", `${g}line l (180,100) -> v 100\n`)).toEqual([]);
});
