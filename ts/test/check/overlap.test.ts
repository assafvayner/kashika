import { expect, it } from "vitest";

import { check } from "../../src/check/index.js";
import { toJsonString } from "../../src/diag/json.js";
import { lint, scene } from "./common/mod.js";

it("overlapping_boxes", () => {
  const d = lint("overlap", "box a at (0,0) size (100,100)\nbox b at (50,50) size (100,100)\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("box 'b' overlaps box 'a'");
  expect(toJsonString(d[0]!.facts.get("intersection")!)).toBe("[[50.0,50.0],[100.0,100.0]]");
  expect(
    lint("overlap", "box a at (0,0) size (100,100)\nbox b at (100,0) size (100,100)\n"),
  ).toEqual([]);
});

it("containment_is_overlap_unless_the_container_is_a_group", () => {
  const d = lint("overlap", "box a at (0,0) size (100,100)\nbox b at (10,10) size (20,20)\n");
  expect(d.length).toBe(1);
  expect(d[0]!.help[0]!.startsWith("only a group may contain")).toBe(true);
  expect(
    lint(
      "overlap",
      "group g at (0,0) size (100,100) {\n  box b at (10,30) size (20,20)\n}\nbox c at (50,50) size (20,20)\n",
    ),
  ).toEqual([]);
  expect(
    lint("overlap", "group g at (0,0) size (100,100) {\n}\nbox c at (90,50) size (20,20)\n").length,
  ).toBe(1);
});

it("shape_allow_suppresses_overlap", () => {
  const s = scene(
    'box a at (0,0) size (100,100)\nbox b at (50,50) size (100,100) allow=overlap:a reason="badge"\n',
  );
  const d = check(s).filter((d) => d.code === "overlap");
  expect(d[0]!.suppressed).toBe(true);
  expect(d[0]!.reason).toBe("badge");
});
