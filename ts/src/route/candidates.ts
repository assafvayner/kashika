import { type Ctx } from "../check/ctx.js";
import { type Axis, freeIntervals } from "../check/free.js";
import { cmpTuple, fmax, fmin, hypot, round, totalCmp } from "../diag/rust.js";
import {
  Pt,
  Rect,
  TOLERANCE,
  boundaryNearest,
  dot,
  insideRuns,
  fmtNum as num,
  pointInPolygon,
  segHit,
} from "../geom/index.js";
import { Layer, type Scene } from "../model/index.js";
import { type Endpoint, Side } from "./endpoint.js";
import { SQUARE, type Style } from "./index.js";

/// Lanes per axis that detours pick from: those nearest either endpoint.
export const DETOUR_LANES = 16;
/// One-lane plans scored with `kashika check`: the best of a geometric pre-pass.
export const MAX_LANE_PLANS = 32;
/// Detours scored with `kashika check`: the best of a geometric pre-pass.
export const MAX_DETOURS = 32;

/// An `x` or `y` target: relative to a box corner where possible, otherwise a literal.
export interface Lane {
  value: number;
  expr: string | null;
}

/// A `curve via` control point.
export interface Ctrl {
  at: Pt;
  expr: string | null;
}

export type Hop =
  /// Straight to the end point.
  | { kind: "To" }
  /// `hv <end>`
  | { kind: "Hv" }
  /// `vh <end>`
  | { kind: "Vh" }
  | { kind: "X"; value: Lane }
  | { kind: "Y"; value: Lane }
  | { kind: "Curve"; left: boolean; amount: number }
  | { kind: "Via"; value: [Ctrl, Ctrl] };

export const Hop = {
  To: { kind: "To" } as Hop,
  Hv: { kind: "Hv" } as Hop,
  Vh: { kind: "Vh" } as Hop,
  X(value: Lane): Hop {
    return { kind: "X", value };
  },
  Y(value: Lane): Hop {
    return { kind: "Y", value };
  },
};

function laneEq(a: Lane, b: Lane): boolean {
  return a.value === b.value && a.expr === b.expr;
}

function ctrlEq(a: Ctrl, b: Ctrl): boolean {
  return a.at.eq(b.at) && a.expr === b.expr;
}

export function hopEq(a: Hop, b: Hop): boolean {
  switch (a.kind) {
    case "To":
    case "Hv":
    case "Vh":
      return a.kind === b.kind;
    case "X":
    case "Y":
      return a.kind === b.kind && laneEq(a.value, (b as typeof a).value);
    case "Curve":
      return b.kind === "Curve" && a.left === b.left && a.amount === b.amount;
    case "Via":
      return b.kind === "Via" && ctrlEq(a.value[0], b.value[0]) && ctrlEq(a.value[1], b.value[1]);
  }
}

export function hopsEq(a: readonly Hop[], b: readonly Hop[]): boolean {
  return a.length === b.length && a.every((h, i) => hopEq(h, b[i]!));
}

export interface Plan {
  hops: Hop[];
  /// The lane the path is routed along: `["x", v]` when a vertical segment runs at x=v.
  /// For a detour, its second lane.
  lane: [Axis, number] | null;
}

/// How a plan's lanes are anchored, lower first: lanes on a box the path neither starts nor
/// ends at, which move with a bystander, then literal lanes, which move with nothing.
export type LaneAnchors = [number, number];

export function laneAnchors(plan: Plan, a: Endpoint, b: Endpoint): LaneAnchors {
  const owns = (id: string) => [a, b].some((e) => e.owner === id);
  return plan.hops
    .flatMap((h) => (h.kind === "X" || h.kind === "Y" ? [h.value] : []))
    .reduce<LaneAnchors>(
      ([bystander, literal], l) => {
        if (l.expr !== null && owns(l.expr.split(".")[0]!)) {
          return [bystander, literal];
        } else if (l.expr !== null) {
          return [bystander + 1, literal];
        } else {
          return [bystander, literal + 1];
        }
      },
      [0, 0],
    );
}

/// A plan's path key, for comparing which plans draw the same path.
export type Key = [number, number][];

function keyStr(k: Key): string {
  return k.map(([x, y]) => `${x},${y}`).join(";");
}

/// The plans worth scoring, without two that draw the same path. Lines in `skip` aren't
/// obstacles when picking lanes.
export function generate(
  ctx: Ctx,
  from: Endpoint,
  to: Endpoint,
  style: Style,
  skip: readonly string[],
): Plan[] {
  const plans = style === "bezier" ? curves(from, to) : orthogonal(ctx, from, to, skip);
  const seen = new Set<string>();
  return plans.filter((p) => {
    const pts = corners(from.at, to.at, p);
    if (pts === null) {
      return true;
    }
    const k = keyStr(key(pts));
    const fresh = !seen.has(k);
    if (fresh) {
      seen.add(k);
    }
    return fresh;
  });
}

/// The corners of an orthogonal plan from `a` to `b`; `null` for curves.
export function corners(a: Pt, b: Pt, plan: Plan): Pt[] | null {
  const pts = [a];
  for (const hop of plan.hops) {
    const at = pts[pts.length - 1]!;
    switch (hop.kind) {
      case "To":
        pts.push(b);
        break;
      case "Hv":
        pts.push(new Pt(b.x, at.y), b);
        break;
      case "Vh":
        pts.push(new Pt(at.x, b.y), b);
        break;
      case "X":
        pts.push(new Pt(hop.value.value, at.y));
        break;
      case "Y":
        pts.push(new Pt(at.x, hop.value.value));
        break;
      case "Curve":
      case "Via":
        return null;
    }
  }
  return pts;
}

/// Whether a path through `pts` has a zero-length segment or one that reverses the one before.
export function doublesBack(pts: readonly Pt[]): boolean {
  for (let i = 0; i + 1 < pts.length; i++) {
    if (pts[i]!.dist(pts[i + 1]!) <= TOLERANCE) return true;
  }
  for (let i = 0; i + 2 < pts.length; i++) {
    const [a, b, c] = [pts[i]!, pts[i + 1]!, pts[i + 2]!];
    if (dot(b.sub(a).norm(), c.sub(b).norm()) < -SQUARE) return true;
  }
  return false;
}

/// Points at 0.1px, without repeats or points in the middle of a straight run, so paths that
/// draw the same shape get the same key.
export function key(pts: Iterable<Pt>): Key {
  const k: Key = [];
  for (const p of pts) {
    const q: [number, number] = [round(p.x * 10.0), round(p.y * 10.0)];
    const last = k[k.length - 1];
    if (last !== undefined && last[0] === q[0] && last[1] === q[1]) {
      continue;
    }
    if (k.length >= 2) {
      const a = k[k.length - 2]!;
      const b = k[k.length - 1]!;
      const u = [b[0] - a[0], b[1] - a[1]] as const;
      const v = [q[0] - b[0], q[1] - b[1]] as const;
      if (u[0] * v[1] === u[1] * v[0] && u[0] * v[0] + u[1] * v[1] > 0) {
        k.pop();
      }
    }
    k.push(q);
  }
  return k;
}

/// The straight path, `hv` and `vh`, the best `MAX_LANE_PLANS` one-lane plans and the best
/// detours. Plans that double back are dropped.
function orthogonal(ctx: Ctx, a: Endpoint, b: Endpoint, skip: readonly string[]): Plan[] {
  const simple: Hop[][] = [];
  if (Math.abs(a.at.x - b.at.x) <= 0.5 || Math.abs(a.at.y - b.at.y) <= 0.5) {
    simple.push([Hop.To]);
  }
  simple.push([Hop.Hv]);
  simple.push([Hop.Vh]);
  const out: Plan[] = simple
    .map((hops): Plan => ({ hops, lane: null }))
    .filter((p) => {
      const pts = corners(a.at, b.at, p);
      return !(pts !== null && doublesBack(pts));
    });
  const field = Field.new(ctx, a, b, skip);
  const found: [Estimate, Plan][] = [];
  for (const [axis, cross] of [
    ["x", Hop.Vh],
    ["y", Hop.Hv],
  ] as [Axis, Hop][]) {
    for (const lane of lanePool(ctx, a, b, axis, skip)) {
      const hops = [hop(axis, lane), cross];
      field.consider(found, hops, [axis, lane.value]);
    }
  }
  out.push(...best(found, MAX_LANE_PLANS));
  out.push(...detours(ctx, field, a, b, skip));
  return out;
}

/// The `n` plans with the best estimates, in that order; ties keep the order they were found.
function best(found: [Estimate, Plan][], n: number): Plan[] {
  found.sort((p, q) => cmpEstimate(p[0], q[0]));
  return found.slice(0, n).map(([, plan]) => plan);
}

/// Three- and four-elbow plans through two or three lanes: `x -> y -> hv` and `y -> x -> vh`
/// end on the other axis than they start, `x -> y -> x -> vh` and `y -> x -> y -> hv` on the
/// same one. A geometric pre-pass drops paths that enter a box, double back, or leave or enter
/// a side anchor the wrong way, and keeps the `MAX_DETOURS` best by estimated crossings,
/// near-misses, segments and length.
function detours(
  ctx: Ctx,
  field: Field,
  a: Endpoint,
  b: Endpoint,
  skip: readonly string[],
): Plan[] {
  const xs = detourLanes(ctx, a, b, "x", skip);
  const ys = detourLanes(ctx, a, b, "y", skip);
  const found: [Estimate, Plan][] = [];
  for (const [lead, back] of [
    ["x", "y"],
    ["y", "x"],
  ] as [Axis, Axis][]) {
    const [outer, inner] = lead === "x" ? [xs, ys] : [ys, xs];
    const [cross, along] = lead === "x" ? [Hop.Hv, Hop.Vh] : [Hop.Vh, Hop.Hv];
    for (const l1 of outer) {
      for (const l2 of inner) {
        const lane: [Axis, number] = [back, l2.value];
        field.consider(found, [hop(lead, l1), hop(back, l2), cross], lane);
        for (const l3 of outer) {
          field.consider(found, [hop(lead, l1), hop(back, l2), hop(lead, l3), along], lane);
        }
      }
    }
  }
  return best(found, MAX_DETOURS);
}

export function hop(axis: Axis, lane: Lane): Hop {
  const l = { value: lane.value, expr: lane.expr };
  return axis === "x" ? Hop.X(l) : Hop.Y(l);
}

/// Lanes on one axis: those of `lanes`, plus the middle of each free interval (as
/// `inspect --corridors` reports between the endpoints) that holds none of them other than a
/// group's, at least `clearance` inside the canvas.
export function lanePool(
  ctx: Ctx,
  a: Endpoint,
  b: Endpoint,
  axis: Axis,
  skip: readonly string[],
): Lane[] {
  const scene = ctx.scene;
  const [band, limit]: [[number, number], number] =
    axis === "x" ? [[a.at.y, b.at.y], scene.size[0]] : [[a.at.x, b.at.x], scene.size[1]];
  const margin = fmax(scene.clearance, TOLERANCE);
  const onGroup = (l: Lane) => {
    if (l.expr === null) return false;
    const id = l.expr.split(".")[0]!;
    return scene.shapes.some((s) => s.id === id && s.kind === "group");
  };
  const pool = lanes(scene, a, b, axis);
  for (const [lo, hi] of freeIntervals(ctx, axis, band, [0.0, limit], skip)) {
    if (pool.some((l) => lo <= l.value && l.value <= hi && !onGroup(l))) {
      continue;
    }
    let v = snap(scene, (lo + hi) / 2.0);
    v = lo <= v && v <= hi ? v : (lo + hi) / 2.0;
    if (v >= margin && v <= limit - margin) {
      pool.push({ value: v, expr: null });
    }
  }
  return pool;
}

/// Lanes for detours on one axis: the `lanePool` lanes at least `clearance` inside the
/// canvas. The `DETOUR_LANES` nearest either endpoint are kept.
export function detourLanes(
  ctx: Ctx,
  a: Endpoint,
  b: Endpoint,
  axis: Axis,
  skip: readonly string[],
): Lane[] {
  const scene = ctx.scene;
  const [pa, pb, limit] =
    axis === "x" ? [a.at.x, b.at.x, scene.size[0]] : [a.at.y, b.at.y, scene.size[1]];
  const margin = fmax(scene.clearance, TOLERANCE);
  let pool = lanePool(ctx, a, b, axis, skip);
  pool = pool.filter((l) => l.value >= margin && l.value <= limit - margin);
  pool.sort((p, q) => cmpTuple([p.expr === null], [q.expr === null]));
  const out: Lane[] = [];
  for (const l of pool) {
    if (!out.some((o) => Math.abs(o.value - l.value) <= 0.5)) {
      out.push(l);
    }
  }
  const away = (v: number) => fmin(Math.abs(v - pa), Math.abs(v - pb));
  out.sort((p, q) => totalCmp(away(p.value), away(q.value)));
  return out.slice(0, DETOUR_LANES);
}

function snap(scene: Scene, v: number): number {
  const grid = scene.grid;
  if (grid !== null && grid > 0.0) {
    return round(v / grid) * grid;
  }
  return round(v);
}

/// Estimated crossings and group transits, near-misses and segments, then length: the order
/// `route` ranks by.
export type Estimate = [number, number, number, number];

function cmpEstimate(a: Estimate, b: Estimate): number {
  return cmpTuple([a[0], a[1], a[2]], [b[0], b[1], b[2]]) || totalCmp(a[3], b[3]);
}

/// The obstacles a plan's corners are measured against.
export class Field {
  private constructor(
    private boxes: [string, Rect, Pt[]][],
    /// Groups that hold neither endpoint: passing through one is a transit.
    private groups: Pt[][],
    private pieces: [Pt, Pt][],
    private starts: string[],
    private ends: string[],
    readonly from: Endpoint,
    readonly to: Endpoint,
    private clearance: number,
  ) {}

  static new(ctx: Ctx, from: Endpoint, to: Endpoint, skip: readonly string[]): Field {
    return new Field(
      ctx.boxes().map((s): [string, Rect, Pt[]] => [s.shape.id, s.shape.rect, s.poly]),
      ctx
        .groups()
        .map((g) => g.poly)
        .filter(
          (poly) =>
            ![from.at, to.at].some(
              (p) => pointInPolygon(p, poly) || boundaryNearest(p, poly)[0] <= TOLERANCE,
            ),
        ),
      ctx.lines
        .filter((l) => !skip.includes(l.line.id))
        .flatMap((l) => l.pieces.map(([p, q]): [Pt, Pt] => [p, q])),
      ctx.attached(from.at),
      ctx.attached(to.at),
      from,
      to,
      ctx.scene.clearance,
    );
  }

  /// Adds `hops` to `found` with its estimate, unless `Field.estimate` rules it out.
  consider(found: [Estimate, Plan][], hops: Hop[], lane: [Axis, number]): void {
    const plan: Plan = { hops, lane };
    const pts = corners(this.from.at, this.to.at, plan);
    if (pts === null) throw new Error("orthogonal hops");
    const e = this.estimate(pts);
    if (e !== null) {
      found.push([e, plan]);
    }
  }

  /// `null` when the path doubles back, leaves or enters a side anchor the wrong way, or
  /// enters a box. The first and last segments may touch the boxes they attach to. Each
  /// group the path passes through counts as a crossing.
  estimate(pts: readonly Pt[]): Estimate | null {
    const n = pts.length - 1;
    if (doublesBack(pts)) {
      return null;
    }
    const dir = (i: number) => pts[i + 1]!.sub(pts[i]!).norm();
    const fs = this.from.side;
    const ts = this.to.side;
    if (
      (fs !== null && dot(dir(0), Side.outward(fs)) <= SQUARE) ||
      (ts !== null && dot(dir(n - 1), Side.outward(ts)) >= -SQUARE)
    ) {
      return null;
    }
    const c = this.clearance;
    let crossings = 0;
    let near = 0;
    let len = 0.0;
    for (let i = 0; i < n; i++) {
      const p = pts[i]!;
      const q = pts[i + 1]!;
      len += p.dist(q);
      const lo = new Pt(fmin(p.x, q.x), fmin(p.y, q.y));
      const hi = new Pt(fmax(p.x, q.x), fmax(p.y, q.y));
      for (const [id, r, poly] of this.boxes) {
        if ((i === 0 && this.starts.includes(id)) || (i === n - 1 && this.ends.includes(id))) {
          continue;
        }
        if (hi.x <= r.x - c || lo.x >= r.right() + c || hi.y <= r.y - c || lo.y >= r.bottom() + c) {
          continue;
        }
        if (insideRuns(p, q, poly).length > 0) {
          return null;
        }
        near += 1;
      }
      for (const [u, v] of this.pieces) {
        const h = segHit(p, q, u, v, 0.0);
        if (h === null) continue;
        if (h.kind === "point") {
          if (h.at.dist(this.from.at) > TOLERANCE && h.at.dist(this.to.at) > TOLERANCE) {
            crossings += 1;
          }
        } else {
          near += 1;
        }
      }
    }
    crossings += this.groups.filter((poly) => {
      for (let i = 0; i + 1 < pts.length; i++) {
        if (insideRuns(pts[i]!, pts[i + 1]!, poly).length > 0) return true;
      }
      return false;
    }).length;
    return [crossings, near, n, len];
  }
}

/// Lane positions on one axis, inside the canvas: the midpoint between the endpoints, and
/// `2 × clearance` outside each box or group edge, written relative to its corner so the lane
/// follows it. Of lanes within 0.5px of each other only one is kept: one on a box the
/// path starts or ends at, otherwise the box nearest the endpoints, before the midpoint. They
/// come in that order of preference.
function lanes(scene: Scene, a: Endpoint, b: Endpoint, axis: Axis): Lane[] {
  const g = 2.0 * scene.clearance;
  const gap = num(g);
  const [pa, pb, limit] =
    axis === "x" ? [a.at.x, b.at.x, scene.size[0]] : [a.at.y, b.at.y, scene.size[1]];
  const ends = Rect.fromPoints(a.at, b.at);
  const owns = (id: string) => [a, b].some((e) => e.owner === id);
  let pool: [Rank, Lane][] = [[[true, true, 0.0], { value: snap(scene, (pa + pb) / 2.0), expr: null }]];
  for (const s of scene.shapes.filter((s) => Layer.isNormal(s.layer))) {
    const r = s.rect;
    const id = s.id;
    const rank: Rank = [false, !owns(id), gapBetween(r, ends)];
    const [before, after]: [[number, string], [number, string]] =
      axis === "x"
        ? [
            [r.x - g, `${id}.tl+(-${gap},0)`],
            [r.right() + g, `${id}.tr+(${gap},0)`],
          ]
        : [
            [r.y - g, `${id}.tl+(0,-${gap})`],
            [r.bottom() + g, `${id}.bl+(0,${gap})`],
          ];
    for (const [value, expr] of [before, after]) {
      pool.push([rank, { value, expr }]);
    }
  }
  pool = pool.filter(([, l]) => l.value > 0.0 && l.value < limit);
  pool.sort((p, q) => totalCmp(p[1].value, q[1].value));
  const kept: [Rank, Lane][] = [];
  let first = -Infinity;
  for (const [rank, lane] of pool) {
    const last = kept[kept.length - 1];
    if (last !== undefined && lane.value - first <= 0.5) {
      if (cmpTuple(rank, last[0]) < 0) {
        kept[kept.length - 1] = [rank, lane];
      }
    } else {
      first = lane.value;
      kept.push([rank, lane]);
    }
  }
  kept.sort((p, q) => cmpTuple(p[0], q[0]));
  return kept.map(([, l]) => l);
}

/// How a lane ranks among lanes at the same position, lower first: whether it is a literal,
/// whether it is on a box other than an endpoint's, and that box's distance from the endpoints.
type Rank = [boolean, boolean, number];

/// The distance between two rects, 0 when they touch or overlap.
function gapBetween(r: Rect, s: Rect): number {
  const dx = fmax(fmax(s.x - r.right(), r.x - s.right()), 0.0);
  const dy = fmax(fmax(s.y - r.bottom(), r.y - s.bottom()), 0.0);
  return hypot(dx, dy);
}

function curves(a: Endpoint, b: Endpoint): Plan[] {
  const d = a.at.dist(b.at);
  if (d <= TOLERANCE) {
    return [];
  }
  const out: Plan[] = [];
  for (const amount of [round(d * 0.15), round(d * 0.3)]) {
    if (amount < 1.0) {
      continue;
    }
    for (const left of [true, false]) {
      const c = bulge(a.at, b.at, left, amount);
      if (square(a, b, c, c)) {
        out.push({ hops: [{ kind: "Curve", left, amount }], lane: null });
      }
    }
  }
  if (a.side !== null || b.side !== null) {
    const k = round(fmax(d / 3.0, 20.0));
    out.push({ hops: [{ kind: "Via", value: [ctrl(a, k), ctrl(b, k)] }], lane: null });
  }
  return out;
}

/// The control point of `curve left|right <amount>` from `a` to `b`.
function bulge(a: Pt, b: Pt, left: boolean, amount: number): Pt {
  const d = b.sub(a).norm();
  const normal = left ? new Pt(d.y, -d.x) : new Pt(-d.y, d.x);
  return a.lerp(b, 0.5).add(normal.mul(2.0 * amount));
}

/// Whether a curve from `a` that leaves toward `c1` and reaches `b` from `c2` leaves and
/// enters side anchors along their normals.
function square(a: Endpoint, b: Endpoint, c1: Pt, c2: Pt): boolean {
  return (
    (a.side === null || dot(c1.sub(a.at).norm(), Side.outward(a.side)) > SQUARE) &&
    (b.side === null || dot(b.at.sub(c2).norm(), Side.outward(b.side)) < -SQUARE)
  );
}

/// A control point `k` px out from an endpoint along its side's normal, written as an offset
/// from the endpoint's reference, folding in the reference's own offset.
function ctrl(e: Endpoint, k: number): Ctrl {
  if (e.side === null) {
    return { at: e.at, expr: e.expr };
  }
  const off = Side.outward(e.side).mul(k);
  const expr =
    e.anchor === null
      ? null
      : (() => {
          const [base, o] = e.anchor;
          const d = o.add(off);
          return `${base}+(${num(d.x)},${num(d.y)})`;
        })();
  return { at: e.at.add(off), expr };
}
