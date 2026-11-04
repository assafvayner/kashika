import { expect, it } from "vitest";

import { Ctx } from "../../src/check/index.js";
import { Pt, fmtNum as num } from "../../src/geom/index.js";
import { Options, type Scene, compile } from "../../src/model/index.js";
import {
  type Hop,
  Hop as H,
  MAX_DETOURS,
  MAX_LANE_PLANS,
  type Plan,
  corners,
  doublesBack,
  generate,
  hopEq,
  hopsEq,
  key,
} from "../../src/route/candidates.js";
import { resolveEndpoints } from "../../src/route/endpoint.js";
import { Endpoint, type Style } from "../../src/route/index.js";
import { pathText, statement } from "../../src/route/text.js";

const WALL =
  'diagram "w" size (460,240)\n' +
  'box a "A" at (20,100) size (100,40)\n' +
  'box wall "W" at (200,40) size (40,160)\n' +
  'box b "B" at (320,100) size (100,40)\n';

const DIAG =
  'diagram "c" size (420,220)\n' +
  "box a at (20,20) size (100,40)\n" +
  "box b at (300,160) size (100,40)\n";

function scene(src: string): Scene {
  const [s, d] = compile(src);
  expect(d).toEqual([]);
  return s;
}

function ends(src: string, from: string, to: string): [Endpoint, Endpoint] {
  const r = resolveEndpoints(src, from, to, new Options());
  if (!r.ok) throw new Error(JSON.stringify(r.error));
  return r.value;
}

function planFor(src: string, from: string, to: string, style: Style): [Endpoint, Endpoint, Plan[]] {
  const s = scene(src);
  const [a, b] = ends(src, from, to);
  const plans = generate(Ctx.new(s), a, b, style, []);
  return [a, b, plans];
}

it("resolves_points_through_the_model", () => {
  const [a, b] = ends(WALL, "a.top", "(300,20)");
  expect(a.at).toEqual(new Pt(70.0, 100.0));
  expect(a.expr).toBe("a.top");
  expect([a.owner, a.side]).toEqual(["a", "top"]);
  expect(b.at).toEqual(new Pt(300.0, 20.0));
  expect([b.expr, b.owner, b.side]).toEqual([null, null, null]);
});

it("endpoints_parse_like_points_in_a_line", () => {
  const src = `${WALL}junction fan at (160,20)\n`;
  let [a, b] = ends(src, "a.right + (0, 4)", "b.left@0.25-(0,4)");
  expect(a.expr).toBe("a.right + (0, 4)");
  expect([a.owner, a.side]).toEqual(["a", "right"]);
  expect(a.anchor).toEqual(["a.right", new Pt(0.0, 4.0)]);
  expect(a.at).toEqual(new Pt(120.0, 124.0));
  expect([b.owner, b.side]).toEqual(["b", "left"]);
  expect(b.anchor).toEqual(["b.left@0.25", new Pt(0.0, -4.0)]);
  expect(b.at).toEqual(new Pt(320.0, 106.0));

  [a, b] = ends(src, "fan", "b.top-left");
  expect([a.owner, a.side]).toEqual(["fan", null]);
  expect([b.owner, b.side]).toEqual(["b", null]);

  [a] = ends(src, "(b.left.x, 30)", "b.top");
  expect(a.expr).toBe("(b.left.x, 30)");
  expect([a.owner, a.side]).toEqual([null, null]);
  expect(a.at).toEqual(new Pt(320.0, 30.0));

  [a, b] = ends(src, "(a.right+(0,4))+(0,2)", "((b.left.x, b.top.y)+(0,20))");
  expect(a.expr).toBe("(a.right+(0,4))+(0,2)");
  expect(a.anchor).toEqual(["a.right", new Pt(0.0, 6.0)]);
  expect(a.at).toEqual(new Pt(120.0, 126.0));
  expect(b.expr).toBe("((b.left.x, b.top.y)+(0,20))");
  expect(b.at).toEqual(new Pt(320.0, 120.0));

  [a] = ends(src, "(10,20)+(5,0)", "b.top");
  expect([a.expr, a.owner]).toEqual([null, null]);
  expect(a.at).toEqual(new Pt(15.0, 20.0));
});

it("bad_points_report_diagnostics_or_usage", () => {
  const opts = new Options();
  const r = resolveEndpoints(WALL, "a.tpo", "b.top", opts);
  if (r.ok || r.error.kind !== "Diagnostics") throw new Error(JSON.stringify(r));
  expect(r.error.diags[0]!.code).toBe("unknown-anchor");
  for (const [from, to] of [
    ["a.top -> (10,10)", "b.top"],
    ["a.top", "b.top\nbox z at (0,0) size (4,4)"],
  ] as const) {
    const r = resolveEndpoints(WALL, from, to, opts);
    expect(!r.ok && r.error.kind === "Usage", `${from} -> ${to}`).toBe(true);
  }
});

it("orthogonal_plans_include_lanes_around_boxes", () => {
  const [a, b, plans] = planFor(WALL, "a.top", "b.top", "corner");
  const lane = H.Y({ value: 28.0, expr: "wall.tl+(0,-12)" });
  const over = plans.find((p) => p.hops[0] !== undefined && hopEq(p.hops[0], lane));
  expect(over, "a lane above the wall").toBeDefined();
  expect(pathText(a, b, over!, Pt.default())).toBe("a.top -> y wall.tl+(0,-12) -> hv b.top");
  const [, , aligned] = plansAligned();
  expect(
    aligned.some((p) => hopsEq(p.hops, [H.To])),
    "aligned endpoints get a straight plan",
  ).toBe(true);
});

function plansAligned(): [Endpoint, Endpoint, Plan[]] {
  return planFor(WALL, "a.right", "b.left", "corner");
}

it("plans_that_double_back_are_dropped", () => {
  expect(doublesBack([new Pt(0.0, 0.0), new Pt(0.0, 0.0), new Pt(10.0, 0.0)])).toBe(true);
  expect(doublesBack([new Pt(0.0, 0.0), new Pt(10.0, 0.0), new Pt(4.0, 0.0)])).toBe(true);
  expect(doublesBack([new Pt(0.0, 0.0), new Pt(10.0, 0.0), new Pt(10.0, 4.0)])).toBe(false);

  const [a, b, plans] = plansAligned();
  expect(plans.length).toBeGreaterThan(0);
  for (const p of plans) {
    const pts = corners(a.at, b.at, p)!;
    expect(doublesBack(pts), pathText(a, b, p, Pt.default())).toBe(false);
  }
  const laneAtEnds = H.Y({ value: 120.0, expr: null });
  expect(plans.some((p) => p.hops[0] !== undefined && hopEq(p.hops[0], laneAtEnds))).toBe(false);
});

it("plans_draw_distinct_paths", () => {
  for (const [src, from, to] of [
    [WALL, "a.top", "b.top"],
    [WALL, "a.right", "b.left"],
    [DIAG, "a.bottom", "b.left"],
  ] as const) {
    const [a, b, plans] = planFor(src, from, to, "corner");
    const keys = plans.map((p) => JSON.stringify(key(corners(a.at, b.at, p)!)));
    expect(new Set(keys).size, `${from} -> ${to}`).toBe(keys.length);
  }
});

it("lanes_at_one_position_keep_the_endpoint_box", () => {
  const src =
    'diagram "l" size (420,300)\n' +
    "box c at (300,20) size (60,40)\n" +
    "box a at (20,20) size (60,40)\n" +
    "box b at (300,200) size (60,40)\n";
  const [a, b, plans] = planFor(src, "a.right", "b.left", "corner");
  const lanes: string[] = plans.flatMap((p) => {
    const h: Hop | undefined = p.hops[0];
    return h !== undefined && h.kind === "X" && p.hops.length === 2 && h.value.value === 288.0
      ? [pathText(a, b, p, Pt.default())]
      : [];
  });
  expect(lanes).toEqual(["a.right -> x b.tl+(-12,0) -> vh b.left"]);
});

it("bezier_plans_skip_curves_that_cannot_leave_square", () => {
  let [a, b, plans] = planFor(DIAG, "a.right", "b.left", "bezier");
  let texts = plans.map((p) => pathText(a, b, p, Pt.default()));
  expect(texts).toEqual(["a.right -> curve via a.right+(76,0) b.left+(-76,0) b.left"]);

  [a, b, plans] = planFor(DIAG, "a.tr", "b.bl", "bezier");
  texts = plans.map((p) => pathText(a, b, p, Pt.default()));
  expect(texts).toEqual([
    "a.tr -> curve left 38 b.bl",
    "a.tr -> curve right 38 b.bl",
    "a.tr -> curve left 76 b.bl",
    "a.tr -> curve right 76 b.bl",
  ]);

  [a, b, plans] = planFor(DIAG, "a.right+(0,4)", "b.left", "bezier");
  expect(pathText(a, b, plans[0]!, Pt.default())).toBe(
    "a.right+(0,4) -> curve via a.right+(75,4) b.left+(-75,0) b.left",
  );
});

it("literals_are_written_in_the_line_frame", () => {
  const from = Endpoint.literal(new Pt(120.0, 140.0));
  const to = Endpoint.literal(new Pt(280.0, 160.0));
  to.expr = "b.left";
  to.owner = "b";
  to.side = "left";
  const plan = (hops: Hop[]): Plan => ({ hops, lane: null });
  const lane = (value: number) => ({ value, expr: null });
  const origin = new Pt(100.0, 100.0);
  expect(pathText(from, to, plan([H.X(lane(220.0)), H.Vh]), origin)).toBe(
    "(20,40) -> x 120 -> vh b.left",
  );
  expect(pathText(from, to, plan([H.X(lane(200.0)), H.Vh]), origin)).toBe(
    "(20,40) -> hvh b.left",
  );
  expect(pathText(from, to, plan([H.Y(lane(150.0)), H.Hv]), origin)).toBe(
    "(20,40) -> vhv b.left",
  );
  const detour = [H.X(lane(60.0)), H.Y(lane(300.0)), H.X(lane(170.0)), H.Vh];
  expect(pathText(from, to, plan(detour), origin)).toBe("(20,40) -> x -40 -> y 200 -> hvh b.left");
});

it("statements_and_numbers", () => {
  expect(statement(false, "ab", "a.right -> b.left", "end=arrow")).toBe(
    "line ab a.right -> b.left end=arrow",
  );
  expect(statement(false, "ab", "a.right -> b.left", "")).toBe("line ab a.right -> b.left");
  expect(statement(true, "ab", "a.right -> b.left", 'label="x"')).toBe(
    'arrow ab a.right -> b.left label="x"',
  );
  expect([num(12.0), num(-0.0), num(0.256)]).toEqual(["12", "0", "0.26"]);
});

/// An `n` × `n` grid of boxes with a line between the first two of each row.
function grid(n: number): string {
  const side = 40 + 120 * n;
  let src = `diagram "big" size (${side},${side})\n`;
  for (let i = 0; i < n; i++) {
    for (let j = 0; j < n; j++) {
      src += `box b${i}_${j} at (${40 + 120 * j},${40 + 120 * i}) size (80,60)\n`;
    }
    src += `line r${i} b${i}_0.right -> b${i}_1.left\n`;
  }
  return src;
}

it("plan_search_stays_bounded_on_a_large_diagram", () => {
  const src = grid(20);
  const s = scene(src);
  const ctx = Ctx.new(s);
  for (const [from, to] of [
    ["b0_0.right", "b19_19.left"],
    ["b0_0.bottom", "b1_0.top"],
  ] as const) {
    const [a, b] = ends(src, from, to);
    const start = performance.now();
    const plans = generate(ctx, a, b, "corner", []);
    const took = performance.now() - start;
    const lanes = plans.filter((p) => p.hops.length === 2).length;
    const detours = plans.filter((p) => p.hops.length >= 3).length;
    expect(lanes, `${lanes} one-lane plans`).toBeLessThanOrEqual(MAX_LANE_PLANS);
    expect(detours, `${detours} detours`).toBeLessThanOrEqual(MAX_DETOURS);
    expect(took, `generating plans took ${took}ms`).toBeLessThan(2000);
  }
});
