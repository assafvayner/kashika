import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("endpoints_closer_than_8px_on_one_shape", () => {
  const d = lint(
    "port-crowding",
    "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50\nline l2 a.top@0.53 -> v -50\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l2 and l1 meet 'a' 3px apart (minimum 8px)");
  expect(
    lint(
      "port-crowding",
      "box a at (100,100) size (100,60)\nline l1 a.top@0.3 -> v -50\nline l2 a.top@0.7 -> v -50\n",
    ),
  ).toEqual([]);
});

it("identical_endpoints_crowd_only_when_both_draw_markers", () => {
  expect(
    lint(
      "port-crowding",
      "box a at (100,100) size (100,60)\nline l1 (0,0) -> hv a.top\nline l2 (300,0) -> hv a.top\n",
    ),
  ).toEqual([]);
  const d = lint(
    "port-crowding",
    "box a at (100,100) size (100,60)\nline l1 (0,0) -> hv a.top end=arrow\nline l2 (300,0) -> hv a.top end=arrow\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l2 and l1 both draw a marker at (150,100) on 'a'");
});

it("junction_endpoints_are_exempt", () => {
  expect(
    lint(
      "port-crowding",
      "box a at (100,100) size (100,60)\njunction j at a.top\nline l1 (0,0) -> hv j end=arrow\nline l2 (300,0) -> hv j end=arrow\n",
    ),
  ).toEqual([]);
});

it("crows_feet_need_room_for_both_spreads", () => {
  const d = lint(
    "port-crowding",
    "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=many\nline l2 a.top@0.6 -> v -50 start=one-many\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l2 and l1 meet 'a' 10px apart (minimum 12px)");
  expect(d[0]!.help[0]!.startsWith("spread the endpoints at least 12px apart")).toBe(true);
  expect(
    lint(
      "port-crowding",
      "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=many\nline l2 a.top@0.62 -> v -50 start=zero-many\n",
    ),
  ).toEqual([]);
  expect(
    lint(
      "port-crowding",
      "box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=arrow\nline l2 a.top@0.6 -> v -50 start=arrow\n",
    ),
  ).toEqual([]);
});

it("markers_on_thick_lines_need_more_room", () => {
  const body = (w: string) =>
    `box a at (100,100) size (100,60)\nline l1 a.top@0.5 -> v -50 start=arrow stroke-width=${w}\nline l2 a.top@0.6 -> v -50 start=arrow stroke-width=${w}\n`;
  expect(lint("port-crowding", body("1.5"))).toEqual([]);
  const d = lint("port-crowding", body("3"));
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l2 and l1 meet 'a' 10px apart (minimum 16px)");
});
