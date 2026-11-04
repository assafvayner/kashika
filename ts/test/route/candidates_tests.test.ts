// Parity of the geometric prefilter, `Field.estimate`, with `kashika check`.
//
// The prefilter drops a plan outright (`null`) for three reasons: it doubles back, it leaves
// or enters a side anchor the wrong way, or it enters a box. The first two are not findings
// of `check` (`rank` drops such paths on its own), so the invariant is about the third:
//
// **A plan the prefilter drops for entering a box is one `check` reports `through` for**, so
// the prefilter never drops a candidate that `check` finds clear of boxes. Its crossing,
// transit and near-miss counts only rank the plans it keeps and are not checked here.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { expect, it } from "vitest";

import { Ctx } from "../../src/check/index.js";
import { type Axis } from "../../src/check/free.js";
import { cmpStr } from "../../src/diag/rust.js";
import { type Pt, dot } from "../../src/geom/index.js";
import { Options, Scene, compileWith } from "../../src/model/index.js";
import {
  Field,
  Hop,
  type Plan,
  corners,
  detourLanes,
  doublesBack,
  hop,
  lanePool,
} from "../../src/route/candidates.js";
import { type Endpoint, PROBE, Side, resolveEndpoints } from "../../src/route/endpoint.js";
import { Frame, SQUARE, astLines, attrs, existing, probeId } from "../../src/route/index.js";
import * as score from "../../src/route/score.js";
import { Source } from "../../src/route/source.js";
import { pathText, statement } from "../../src/route/text.js";
import { parse } from "../../src/syntax/index.js";

/// A line to route, with what `rank` needs to write its probes.
interface Case {
  name: string;
  src: string;
  opts: Options;
  from: Endpoint;
  to: Endpoint;
  /// The line being rerouted, which isn't an obstacle.
  line: string | null;
  arrow: boolean;
  attrs: string;
  frame: Frame;
}

function sceneOf(src: string, opts: Options): Scene {
  return compileWith(src, opts)[0];
}

/// Every named line of every example, rerouted as `kashika route --line` would.
function exampleCases(): Case[] {
  const root = join(import.meta.dirname, "../../../examples");
  const files: string[] = readdirSync(root)
    .map((d) => join(root, d))
    .filter((d) => statSync(d).isDirectory())
    .flatMap((d) => readdirSync(d).map((f) => join(d, f)))
    .filter((p) => p.endsWith(".kash"));
  files.sort(cmpStr);
  const out: Case[] = [];
  for (const path of files) {
    const src = readFileSync(path, "utf8");
    const opts = new Options({ path });
    const scene = sceneOf(src, opts);
    const { file } = parse(src);
    const source = Source.new(src);
    const ids: string[] = astLines(file.stmts).flatMap((l) => (l.id === null ? [] : [l.id.name]));
    for (const id of ids) {
      const t = existing(file, scene, source, id);
      if (!t.ok) {
        continue;
      }
      out.push({
        name: `${path}:${id}`,
        src,
        opts,
        from: t.value.from,
        to: t.value.to,
        line: id,
        arrow: t.value.arrow,
        attrs: attrs(t.value.attrs, "corner", 0.0, false).text(null),
        frame: { origin: t.value.origin, after: t.value.end },
      });
    }
  }
  return out;
}

const SYNTHETIC: [string, string, string][] = [
  [
    'diagram "wall" size (460,240)\n' +
      'box a "A" at (20,100) size (100,40)\n' +
      'box wall "W" at (200,40) size (40,160)\n' +
      'box b "B" at (320,100) size (100,40)\n',
    "a.right",
    "b.left",
  ],
  [
    'diagram "maze" size (520,320)\n' +
      "box a at (20,20) size (80,40)\n" +
      "box p at (140,0) size (40,200)\n" +
      "box q at (260,120) size (40,200)\n" +
      "box r at (360,40) size (120,30) shape=diamond\n" +
      "box b at (400,240) size (80,40)\n",
    "a.bottom",
    "b.top",
  ],
  [
    'diagram "groups" size (520,320)\n' +
      'group g "G" at (160,40) size (200,240) {\n  box inner at (40,60) size (80,40)\n}\n' +
      "box a at (20,140) size (80,40)\n" +
      "box b at (420,140) size (80,40)\n" +
      "line down inner.bottom -> (260,300)\n",
    "a.right",
    "b.left",
  ],
  [
    'diagram "corner" size (400,300)\n' +
      "box a at (20,20) size (100,60)\n" +
      "box b at (260,200) size (100,60)\n" +
      "box mid at (150,110) size (80,60) shape=ellipse\n",
    "(70,80)",
    "b.top",
  ],
];

function syntheticCases(): Case[] {
  return SYNTHETIC.map(([src, from, to], i) => {
    const opts = new Options();
    const [, diags] = compileWith(src, opts);
    expect(diags, `synthetic ${i}`).toEqual([]);
    const r = resolveEndpoints(src, from, to, opts);
    if (!r.ok) throw new Error(JSON.stringify(r.error));
    const [f, t] = r.value;
    return {
      name: `synthetic ${i}`,
      src,
      opts,
      from: f,
      to: t,
      line: null,
      arrow: false,
      attrs: attrs(null, "corner", 0.0, false).text(null),
      frame: Frame.default(),
    };
  });
}

/// The one-lane plans and detours `orthogonal` builds before keeping the best few, with fewer
/// four-elbow detours to keep the test quick.
function allPlans(ctx: Ctx, a: Endpoint, b: Endpoint, skip: readonly string[]): Plan[] {
  const out: Plan[] = [];
  for (const [axis, cross] of [
    ["x", Hop.Vh],
    ["y", Hop.Hv],
  ] as [Axis, Hop][]) {
    for (const lane of lanePool(ctx, a, b, axis, skip)) {
      out.push({ hops: [hop(axis, lane), cross], lane: [axis, lane.value] });
    }
  }
  const xs = detourLanes(ctx, a, b, "x", skip);
  const ys = detourLanes(ctx, a, b, "y", skip);
  for (const [lead, back] of [
    ["x", "y"],
    ["y", "x"],
  ] as [Axis, Axis][]) {
    const [outer, inner] = lead === "x" ? [xs, ys] : [ys, xs];
    const [cross, along] = lead === "x" ? [Hop.Hv, Hop.Vh] : [Hop.Vh, Hop.Hv];
    for (const l1 of outer) {
      for (const l2 of inner) {
        const lane: [Axis, number] = [back, l2.value];
        out.push({ hops: [hop(lead, l1), hop(back, l2), cross], lane });
        for (const l3 of outer.slice(0, 4)) {
          out.push({ hops: [hop(lead, l1), hop(back, l2), hop(lead, l3), along], lane });
        }
      }
    }
  }
  return out;
}

/// Whether the prefilter drops `pts` for entering a box, rather than for its shape.
function droppedForABox(field: Field, pts: readonly Pt[]): boolean {
  const n = pts.length - 1;
  const dir = (i: number) => pts[i + 1]!.sub(pts[i]!).norm();
  const fs = field.from.side;
  const ts = field.to.side;
  const wrongWay =
    (fs !== null && dot(dir(0), Side.outward(fs)) <= SQUARE) ||
    (ts !== null && dot(dir(n - 1), Side.outward(ts)) >= -SQUARE);
  return !doublesBack(pts) && !wrongWay && field.estimate(pts) === null;
}

/// For each plan, whether `check` keeps it clear of `through`, as `rank` scores it; `null`
/// when its probe doesn't resolve.
function clearOfBoxes(c: Case, scene: Scene, plans: readonly Plan[]): (boolean | null)[] {
  const src = Source.new(c.src);
  const probes: string[] = plans.map((p, i) => {
    const path = pathText(c.from, c.to, p, c.frame.origin);
    return statement(c.arrow, probeId(i), path, c.attrs);
  });
  const [text] = src.withLines(c.frame.after, probes);
  const probed = sceneOf(text, c.opts);
  const at = probed.lines.findIndex((l) => l.id.startsWith(PROBE));
  if (at < 0) {
    return plans.map(() => null);
  }
  const work = new Scene({ ...scene, lines: [...scene.lines] });
  return plans.map((_, i) => {
    const line = probed.lines.slice(at).find((l) => l.id === probeId(i));
    if (line === undefined) return null;
    work.lines.splice(at, 0, line);
    const s = score.score(work, work.lines[at]!, c.line);
    work.lines.splice(at, 1);
    return s !== null;
  });
}

/// Probes scored with `check` per case, spread over the dropped plans.
const SAMPLE = 24;

it("the_prefilter_drops_only_plans_check_finds_through_a_box", () => {
  const cases: Case[] = [...syntheticCases(), ...exampleCases()];
  expect(cases.length, `only ${cases.length} cases`).toBeGreaterThan(40);
  let considered = 0;
  let dropped = 0;
  let scored = 0;
  const wrong: string[] = [];
  for (const c of cases) {
    const scene = sceneOf(c.src, c.opts);
    const ctx = Ctx.new(scene);
    const skip: string[] = c.line === null ? [] : [c.line];
    const field = Field.new(ctx, c.from, c.to, skip);
    const plans = allPlans(ctx, c.from, c.to, skip);
    considered += plans.length;
    let rejected = plans.filter((p) => {
      const pts = corners(c.from.at, c.to.at, p);
      if (pts === null) throw new Error("orthogonal hops");
      return droppedForABox(field, pts);
    });
    dropped += rejected.length;
    const step = Math.max(Math.ceil(rejected.length / SAMPLE), 1);
    rejected = rejected.filter((_, i) => i % step === 0);
    const clear = clearOfBoxes(c, scene, rejected);
    rejected.forEach((plan, i) => {
      const cl = clear[i];
      if (cl === null || cl === undefined) return;
      scored += 1;
      if (cl) {
        const path = pathText(c.from, c.to, plan, c.frame.origin);
        wrong.push(`${c.name}: ${path}`);
      }
    });
  }
  expect(dropped > 0 && scored > 0, `nothing dropped of ${considered} plans`).toBe(true);
  expect(
    wrong,
    `the prefilter dropped ${wrong.length} of ${scored} scored plans that check finds clear of boxes`,
  ).toEqual([]);
});
