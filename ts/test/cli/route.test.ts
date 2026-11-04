import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { ROOT, fixture, kashika, tempdir } from "../support/cli.js";

test("prints_ranked_ready_to_paste_candidates", () => {
  const r = kashika(["route", fixture("route.kash"), "a.top", "b.top"]);
  expect([r.code, r.stdout]).toEqual([
    0,
    "#1  crossings=0 transits=0 near=0 segments=3 length=444px  via corridor y 0..34\n    line a_b a.top -> y wall.tl+(0,-12) -> hv b.top end=arrow\n",
  ]);
});

test("style_argument_selects_arc", () => {
  const r = kashika(["route", fixture("route.kash"), "a.top", "b.top", "arc", "--radius", "6"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("end=arrow radius=6");
});

test("json_output", () => {
  const r = kashika(["route", fixture("route.kash"), "a.top", "b.top", "--format", "json"]);
  expect(r.code).toBe(0);
  const v = JSON.parse(r.stdout);
  expect(v.schema).toBe("kashika.route/1");
  expect(v.style).toBe("corner");
  expect(v.from.side).toBe("top");
  expect(v.candidates[0].segments).toBe(3);
  expect(v.candidates[0].statement.startsWith("line a_b ")).toBe(true);
});

test("reroutes_with_line_flag", () => {
  const dir = tempdir();
  const f = join(dir, "r.kash");
  const src = readFileSync(fixture("route.kash"), "utf8");
  writeFileSync(f, `${src}line ab a.top -> y 28 -> hv b.top end=diamond\n`);
  const r = kashika(["route", f, "--line", "ab"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("line ab a.top -> y wall.tl+(0,-12) -> hv b.top end=diamond");
});

test("rerouting_an_arrow_keeps_the_keyword", () => {
  const dir = tempdir();
  const f = join(dir, "r.kash");
  const src = readFileSync(fixture("route.kash"), "utf8");
  writeFileSync(f, `${src}arrow ab a.top -> y 28 -> hv b.top\n`);
  const r = kashika(["route", f, "--line", "ab"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("arrow ab a.top -> y wall.tl+(0,-12) -> hv b.top\n");
});

test("no_path_exits_one", () => {
  const r = kashika(["route", "-", "a.right", "b.left"], {
    input:
      'diagram "b" size (300,200)\nbox a at (20,80) size (100,40)\nbox c at (122,20) size (40,160)\nbox b at (220,80) size (60,40)\n',
  });
  expect(r.code).toBe(1);
  expect(r.stdout).toContain("no candidate path avoids every box with at most 4 elbows");
});

test("detours_when_no_simple_path_is_clear", () => {
  const r = kashika(["route", fixture("route.kash"), "a.right", "b.left", "--max", "1"]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("segments=5");
  expect(r.stdout).toContain("line a_b a.right -> x ");
});

test("unknown_anchor_exits_two", () => {
  const r = kashika(["route", fixture("route.kash"), "a.tpo", "b.top"]);
  expect(r.code).toBe(2);
  expect(r.stdout).toContain("error[unknown-anchor]");
});

test("one_point_is_a_usage_error", () => {
  expect(kashika(["route", fixture("route.kash"), "a.top"]).code).toBe(3);
});

test("unknown_line_is_a_usage_error", () => {
  const r = kashika(["route", fixture("route.kash"), "--line", "nope"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("no line `nope`");
});

test("hints_at_line_with_same_endpoints", () => {
  const ex = join(ROOT, "examples/upload/upload.kash");
  expect(kashika(["route", ex, "api1.bottom", "queue.left"]).stdout).toContain(
    "hint: line `api1_queue` already connects these; pass --line api1_queue",
  );
  expect(kashika(["route", ex, "--line", "api1_queue"]).stdout).not.toContain("hint:");
});

test("notes_allow_entries_a_suggestion_no_longer_needs", () => {
  const dir = tempdir();
  const f = join(dir, "r.kash");
  writeFileSync(
    f,
    'diagram "r" size (460,240)\nbox a "A" at (20,100) size (100,40)\nbox b "B" at (320,100) size (100,40)\nline l_feat (220,60) -> (220,180)\nline ab a.right -> b.left end=arrow allow=cross:l_feat reason="shared bus"\n',
  );
  const r = kashika(["route", f, "--line", "ab", "--max", "1"]);
  expect([r.code, r.stdout]).toEqual([
    0,
    "#1  crossings=0 transits=0 near=0 segments=5 length=386px  via corridor y 0..54\n    line ab a.right -> x a.tr+(12,0) -> y 27 -> x b.tl+(-12,0) -> vh b.left end=arrow\n    note: dropped allow=cross:l_feat (no longer needed)\n",
  ]);
  const v = JSON.parse(kashika(["route", f, "--line", "ab", "--max", "1", "--format", "json"]).stdout);
  expect(v.candidates[0].dropped_allow).toEqual(["cross:l_feat"]);
});
