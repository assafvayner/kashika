import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("collinear_overlap_is_reported_with_the_nearest_free_lane", () => {
  let d = lint(
    "shared-segment",
    "box s at (40,80) size (60,40)\nline a s.right -> h 200\nline b s.right -> h 100 -> v 100\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("b runs along a for 100px, from (100,100) to (200,100)");
  expect(d[0]!.elements[0]!.id).toBe("b");
  expect(d[0]!.elements[1]!.id).toBe("a");
  expect(d[0]!.help).toEqual([
    "nearest free lane: y=74",
    "free y-intervals across x 100..200: 0..74, 126..600",
    "both lines leave or reach the same point: spread them along the side with `side@t` (`a.right@0.3`, `a.right@0.7`), or fan out from a `junction`",
  ]);
  d = lint("shared-segment", "line a (20,100) -> h 200\nline b (100,100) -> h 180\n");
  expect(d[0]!.help.length).toBe(2);
});

it("parallel_or_tiny_overlaps_are_not_shared", () => {
  expect(lint("shared-segment", "line a (0,100) -> h 100\nline b (0,110) -> h 100\n")).toEqual([]);
  expect(lint("shared-segment", "line a (0,100) -> h 100\nline b (99.5,100) -> h 100\n")).toEqual(
    [],
  );
});
