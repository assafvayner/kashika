import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("diagonal_last_hop_after_orthogonal_ones", () => {
  const d = lint("diagonal-tail", "line l (0,0) -> h 100 -> v 50 -> (150,100)\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("the last segment of l runs diagonally from (100,50) to (150,100)");
  expect(d[0]!.elements[0]!.segment).toBe(3);
  expect(lint("diagonal-tail", "line l (0,0) -> (150,100)\n")).toEqual([]);
  expect(lint("diagonal-tail", "line l (0,0) -> (50,20) -> (150,100)\n")).toEqual([]);
  expect(lint("diagonal-tail", "line l (0,0) -> h 100 -> v 50\n")).toEqual([]);
});
