import { expect, it } from "vitest";

import { toJsonString } from "../../src/diag/json.js";
import { lint } from "./common/mod.js";

it("crossing_lines_are_reported_on_the_later_line", () => {
  const d = lint("cross", "line a (100,100) -> h 200\nline b (200,50) -> v 100\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("b crosses a at (200,100)");
  expect(d[0]!.span.line).toBe(3);
  expect(d[0]!.elements[0]!.id).toBe("b");
  expect(d[0]!.elements[0]!.segment).toBe(1);
  expect(d[0]!.elements[1]!.id).toBe("a");
  expect(toJsonString(d[0]!.facts.get("at")!)).toBe("[200.0,100.0]");
  expect(d[0]!.help).toEqual([
    "segment 1 runs at x=200; free x-intervals across y 50..150: 0..94, 306..800",
    "if intentional: allow=cross:a on b",
  ]);
});

it("shared_endpoints_and_junctions_are_not_crossings", () => {
  expect(lint("cross", "line a (0,0) -> (100,100)\nline b (100,100) -> (200,0)\n")).toEqual([]);
  expect(
    lint(
      "cross",
      "junction j at (100,100)\nline a (0,100) -> h 200\nline b (100,0) -> j\nline c j -> v 100\n",
    ),
  ).toEqual([]);
});

it("a_shared_segment_is_not_also_a_crossing", () => {
  expect(
    lint("cross", "line a (100,100) -> h 200\nline b (100,100) -> h 100 -> v 100\n"),
  ).toEqual([]);
});

it("tangency_counts_once_and_curves_take_part", () => {
  let d = lint("cross", "line a (0,100) -> h 200\nline b (50,50) -> curve right 50 (150,50)\n");
  expect(d.length).toBe(1);
  d = lint("cross", "line a (0,100) -> h 200\nline b (50,50) -> curve right 100 (150,50)\n");
  expect(d.length).toBe(2);
});

it("unnamed_lines_are_told_to_take_a_name", () => {
  const d = lint("cross", "line (100,100) -> h 200\nline b (200,50) -> v 100\n");
  expect(d[0]!.help[d[0]!.help.length - 1]).toBe(
    "if intentional: name line@L2 and add allow=cross:<its id> on b, or allow=cross on b",
  );
});
