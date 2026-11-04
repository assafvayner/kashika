import { expect, it } from "vitest";

import { Ctx, checkBounded } from "../../src/check/index.js";
import { type Diagnostic, Span } from "../../src/diag/index.js";
import { lint, scene } from "./common/mod.js";

const CROSSING = "line a (0,50) -> h 100\nline b (50,0) -> v 100\n";

it("reports_each_stale_entry_at_the_entry", () => {
  const d = lint(
    "unused-allow",
    `${CROSSING}line c (0,150) -> h 100 allow=cross:a,near reason="x"\n`,
  );
  expect(d.length).toBe(2);
  expect(d[0]!.message).toBe("`allow=cross:a` on c suppresses nothing");
  expect(d[0]!.span).toEqual(Span.new(4, 31, 4, 38));
  expect(d[1]!.message).toBe("`allow=near` on c suppresses nothing");
  expect(d[1]!.span).toEqual(Span.new(4, 39, 4, 43));
  expect(d.every((d) => !d.suppressed && d.elements[0]!.id === "c")).toBe(true);
  expect(d[0]!.help[0]).toContain("`reason=`");
});

it("a_used_entry_is_not_reported", () => {
  const body = CROSSING.replace("v 100", 'v 100 allow=cross:a,near reason="bus"');
  const d = lint("unused-allow", body);
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("`allow=near` on b suppresses nothing");
  expect(d[0]!.help).toEqual(["remove it"]);
  expect(lint("unused-allow", CROSSING.replace("v 100", "v 100 allow=cross"))).toEqual([]);
});

it("shapes_name_their_kind", () => {
  const d = lint("unused-allow", "box card at (20,20) size (60,40) allow=overlap\n");
  expect(d.length).toBe(1);
  expect(d[0]!.message).toBe("`allow=overlap` on box 'card' suppresses nothing");
});

it("allow_unused_allow_silences_the_others", () => {
  const d = lint("unused-allow", "box card at (20,20) size (60,40) allow=overlap,unused-allow\n");
  expect(d.length).toBe(1);
  expect(d[0]!.suppressed).toBe(true);
  expect(lint("unused-allow", "box card at (20,20) size (60,40) allow=unused-allow\n")).toEqual([]);
});

it("output_limits_do_not_make_entries_unused", () => {
  const s = scene(CROSSING.replace("v 100", "v 100 allow=cross:a"));
  const onlyUnused = (d: Diagnostic) => d.code === "unused-allow";
  let b = checkBounded(Ctx.new(s), 0, onlyUnused);
  expect(b.found("unused-allow")).toBe(0);
  b = checkBounded(Ctx.new(s), 1, (d) => d.elements.some((e) => e.id === "z"));
  expect(b.found("unused-allow")).toBe(0);
});
