import { expect, it } from "vitest";

import { Ctx, check, checkBounded, checkDeferred } from "../../src/check/index.js";
import {
  type Axis,
  DeferredKind,
  corridors,
  freeAround,
  freeIntervals,
  resolve,
  withDeferred,
  withFree,
} from "../../src/check/free.js";
import { Diagnostic, Span } from "../../src/diag/index.js";
import { toJsonString } from "../../src/diag/json.js";
import { Pt, Rect } from "../../src/geom/index.js";
import { scene } from "./common/mod.js";

const X: Axis = "x";
const Y: Axis = "y";

it("free_intervals_avoid_boxes_and_lines_with_clearance", () => {
  const s = scene("box a at (100,110) size (80,60)\nline l (0,300) -> h 800\n");
  const ctx = Ctx.new(s);
  expect(freeIntervals(ctx, Y, [50, 500], [0, 600], [])).toEqual([
    [0, 104],
    [176, 294],
    [306, 600],
  ]);
  expect(freeIntervals(ctx, Y, [50, 500], [0, 600], ["l"])).toEqual([
    [0, 104],
    [176, 600],
  ]);
  expect(freeIntervals(ctx, Y, [200, 500], [0, 600], ["l"])).toEqual([[0, 600]]);
});

it("free_intervals_avoid_group_borders_but_not_their_interior", () => {
  const s = scene("group g at (100,100) size (200,200) {\n}\n");
  const ctx = Ctx.new(s);
  expect(freeIntervals(ctx, X, [0, 600], [0, 800], [])).toEqual([
    [0, 94],
    [106, 294],
    [306, 800],
  ]);
  expect(freeIntervals(ctx, X, [400, 600], [0, 800], [])).toEqual([[0, 800]]);
});

it("free_intervals_drop_gaps_that_round_to_nothing", () => {
  const s = scene("box a at (0,0) size (100,100)\nbox b at (0,112.004) size (100,50)\n");
  const ctx = Ctx.new(s);
  expect(freeIntervals(ctx, Y, [10, 90], [0, 600], [])).toEqual([[168, 600]]);
});

it("free_intervals_avoid_texts_labels_and_junctions", () => {
  const freeIn = (body: string, band: [number, number], skip: string[]) => {
    const s = scene(body);
    return freeIntervals(Ctx.new(s), Y, band, [0, 600], skip);
  };
  const free = (body: string, skip: string[]) => freeIn(body, [0, 800], skip);
  const text = free('text "note" at (400,300)\n', []);
  expect(text.length).toBe(2);
  expect(text[0]![1] < 294 && text[1]![0] > 306).toBe(true);
  const junction = free("junction j at (400,300)\n", []);
  expect(junction).toEqual([
    [0, 294],
    [306, 600],
  ]);
  const labeled = 'line l (400,250) -> v 100 label="a wide label"\n';
  const label = freeIn(labeled, [410, 800], []);
  expect(label.length).toBe(2);
  expect(label[0]![1] < 294 && label[1]![0] > 306).toBe(true);
  expect(freeIn(labeled, [410, 800], ["l"])).toEqual([[0, 600]]);
  expect(freeIn("line l (400,250) -> v 100\n", [410, 800], [])).toEqual([[0, 600]]);
});

it("free_intervals_avoid_the_label_under_a_below_shape", () => {
  const s = scene('box b "Below" at (380,200) size (80,40)\n');
  s.shapes[0]!.textBelow = true;
  const label = s.shapes[0]!.text[0]!.rect;
  for (const item of s.shapes[0]!.text) {
    item.rect = new Rect(item.rect.x, item.rect.y + 60, item.rect.w, item.rect.h);
  }
  const ctx = Ctx.new(s);
  const free = freeIntervals(ctx, Y, [0, 800], [0, 600], []);
  expect(free.length).toBe(3);
  expect(free[2]![0] >= label.bottom() + 60).toBe(true);
  const skipped = freeIntervals(ctx, Y, [0, 800], [0, 600], ["b"]);
  expect(skipped).toEqual([[0, 600]]);
});

it("free_around_a_horizontal_piece_and_its_nearest_lane", () => {
  const s = scene("box a at (100,110) size (80,60)\n");
  const ctx = Ctx.new(s);
  const f = freeAround(ctx, new Pt(50, 150), new Pt(300, 150), [])!;
  expect(f.axis).toBe(Y);
  expect(f.describe()).toBe("free y-intervals across x 50..300: 0..104, 176..600");
  expect(f.nearest()).toBe(176);
  expect(toJsonString(f.fact())).toBe(
    '{"axis":"y","band":[50.0,300.0],"intervals":[[0.0,104.0],[176.0,600.0]]}',
  );
  expect(freeAround(ctx, new Pt(0, 0), new Pt(10, 10), [])).toBeNull();
});

it("free_around_a_vertical_piece_spans_the_canvas_width", () => {
  const s = scene("box a at (100,110) size (80,60)\n");
  const ctx = Ctx.new(s);
  const f = freeAround(ctx, new Pt(150, 0), new Pt(150, 400), [])!;
  expect(f.axis).toBe(X);
  expect(f.intervals).toEqual([
    [0, 94],
    [186, 800],
  ]);
  expect(f.nearest()).toBe(186);
});

it("with_free_adds_a_fact_and_help_for_straight_pieces", () => {
  const s = scene("box a at (100,110) size (80,60)\nline l (50,150) -> h 250 -> v 100\n");
  const ctx = Ctx.new(s);
  const l = ctx.line("l")!;
  const f = withFree(Diagnostic.warn("through", Span.default(), "x"), l, 0, ["l"]);
  expect(f.diag.help.length === 0 && f.deferred.length === 1).toBe(true);
  const d = f.resolved(ctx);
  expect(d.help).toEqual([
    "segment 1 runs at y=150; free y-intervals across x 50..300: 0..104, 176..600",
  ]);
  expect(d.facts.has("free")).toBe(true);
});

it("corridors_cover_both_axes", () => {
  const s = scene("box a at (100,100) size (100,100)\n");
  const [h, v] = corridors(s, Rect.new(0, 0, 400, 400));
  expect(h).toEqual([
    [0, 94],
    [206, 400],
  ]);
  expect(v).toEqual([
    [0, 94],
    [206, 400],
  ]);
});

it("deferred_help_resolves_to_what_check_reports", () => {
  const s = scene(
    "box a at (100,100) size (80,60)\nbox b at (400,100) size (80,60)\n" +
      'line lx (0,130) -> h 600 allow=cross reason="bus"\nline ly (300,0) -> v 400\n' +
      "line p (0,300) -> h 400\nline q (100,300) -> h 400\n" +
      "line n (0,163) -> h 150\n",
  );
  const eager = check(s);
  const ctx = Ctx.new(s);
  const lazy = checkBounded(ctx, Infinity, () => true);
  expect(lazy.diags).toEqual(checkDeferred(s));
  const pending = () =>
    lazy.diags
      .map((d, i): [Diagnostic, unknown[]] => [d, lazy.deferred[i]!])
      .filter(([, x]) => x.length > 0);
  const codes = pending().map(([d]) => d.code);
  for (const code of ["cross", "through", "shared-segment", "near"]) {
    expect(codes.includes(code), `${code} not deferred: ${codes}`).toBe(true);
  }
  expect(pending().some(([d]) => d.suppressed)).toBe(true);
  expect(lazy.diags).not.toEqual(eager);
  const resolved = lazy.diags.map((d, i) => {
    resolve(ctx, d, lazy.deferred[i]!);
    return d;
  });
  expect(resolved).toEqual(eager);
});

it("a_lane_with_nowhere_free_lists_no_lane", () => {
  const s = scene("box wall at (0,0) size (800,600)\n");
  const ctx = Ctx.new(s);
  const d = withDeferred(
    Diagnostic.warn("shared-segment", Span.default(), "x"),
    DeferredKind.LANE,
    new Pt(100, 300),
    new Pt(200, 300),
    [],
  ).resolved(ctx);
  expect(d.help).toEqual(["free y-intervals across x 100..200: none"]);
  expect(d.facts.has("lane")).toBe(false);
  expect(toJsonString((d.facts.get("free") as { intervals: never }).intervals)).toBe("[]");
});

it("two_deferred_entries_keep_their_place_in_the_help", () => {
  const s = scene("box a at (100,110) size (80,60)\nline l (50,150) -> h 250 -> v 100\n");
  const ctx = Ctx.new(s);
  const l = ctx.line("l")!;
  const d0 = Diagnostic.warn("through", Span.default(), "x").withHelp("first");
  const f1 = withFree(d0, l, 0, ["l"]).withHelp("second");
  const f = withDeferred(f1, DeferredKind.LANE, new Pt(50, 150), new Pt(300, 150), ["l"]).withHelp(
    "third",
  );
  expect(f.deferred.length).toBe(2);
  const d = f.resolved(ctx);
  expect(d.help).toEqual([
    "first",
    "segment 1 runs at y=150; free y-intervals across x 50..300: 0..104, 176..600",
    "second",
    "nearest free lane: y=176",
    "free y-intervals across x 50..300: 0..104, 176..600",
    "third",
  ]);
  expect(toJsonString(d.facts.get("lane")!)).toBe('{"at":176.0,"axis":"y"}');
});
