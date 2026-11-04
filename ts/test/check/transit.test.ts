import { expect, it } from "vitest";

import { lint } from "./common/mod.js";

it("line_crossing_a_group_it_does_not_connect_to", () => {
  const d = lint(
    "group-transit",
    "group g at (100,0) size (100,100) {\n}\nline l (0,50) -> h 300\n",
  );
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("l passes through group 'g' without connecting to anything inside it");
  expect(
    lint(
      "group-transit",
      "group g at (100,0) size (100,100) {\n  box a at (40,40) size (20,20)\n}\nline l (0,50) -> a.left\n",
    ),
  ).toEqual([]);
});

const TRUNK =
  "box src at (0,40) size (40,20)\n" +
  "group r2 at (100,0) size (100,100) {\n  box b at (30,60) size (40,20)\n}\n" +
  "group r3 at (300,0) size (100,100) {\n  box c at (30,40) size (40,20)\n}\n" +
  "line trunk src.right -> c.left\n";

it("a_trunk_whose_junction_taps_into_the_group_connects_to_it", () => {
  const outside = `${TRUNK}junction mid at (80,50)\nline tap mid -> vh b.left\n`;
  expect(lint("group-transit", outside)).toEqual([]);
  const inside = `${TRUNK}junction mid at (150,50)\nline tap mid -> b.top\n`;
  expect(lint("group-transit", inside)).toEqual([]);
});

it("a_trunk_without_a_tap_into_the_group_still_transits_it", () => {
  const d = lint("group-transit", TRUNK);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe(
    "trunk passes through group 'r2' without connecting to anything inside it",
  );
  const elsewhere = `${TRUNK}box e at (60,200) size (40,20)\njunction mid at (80,50)\nline tap mid -> e.top\n`;
  expect(lint("group-transit", elsewhere).length).toBe(1);
  const offTheTrunk = `${TRUNK}junction mid at (80,90)\nline tap mid -> h 50\n`;
  expect(lint("group-transit", offTheTrunk).length).toBe(1);
});
