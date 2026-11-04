import { type Diagnostic } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { fclamp, fmax, fmin, totalCmp } from "../diag/rust.js";
import { Pt, Rect, TOLERANCE, fmtNum, round2 } from "../geom/index.js";
import { type Scene } from "../model/index.js";
import { Ctx, type LineGeom } from "./ctx.js";

/// A closed range `lo..hi` on one axis.
export type Interval = [number, number];

/// The axis that free values lie on.
export type Axis = "x" | "y";

export const Axis = {
  name(a: Axis): string {
    return a;
  },
};

/// Values on `axis`, within `within`, where a straight run spanning `band` on the other axis
/// keeps `clearance` from every box, image, line piece, junction, text, line label and label
/// under a `text-area=below` shape, and from every group border it runs along. Ids in `skip`
/// are ignored, along with the labels of skipped lines and shapes.
export function freeIntervals(
  ctx: Ctx,
  axis: Axis,
  band: Interval,
  within: Interval,
  skip: readonly string[],
): Interval[] {
  const c = ctx.scene.clearance;
  const [b0, b1] = [fmin(band[0], band[1]), fmax(band[0], band[1])];
  const big = Number.MAX_VALUE / 4.0;
  const strip =
    axis === "y" ? new Rect(b0, -big, b1 - b0, 2.0 * big) : new Rect(-big, b0, 2.0 * big, b1 - b0);
  const blocked: Interval[] = [];
  for (const g of ctx.groupsNear(strip, 0.0)) {
    if (skip.includes(g.shape.id)) {
      continue;
    }
    const r = g.shape.rect;
    const [across, sides]: [Interval, [number, number]] =
      axis === "y" ? [[r.x, r.right()], [r.y, r.bottom()]] : [[r.y, r.bottom()], [r.x, r.right()]];
    if (across[0] < b1 - TOLERANCE && across[1] > b0 + TOLERANCE) {
      blocked.push(...sides.map((v): Interval => [v - c, v + c]));
    }
  }
  const block = (r: Rect) => {
    const [across, along]: [Interval, Interval] =
      axis === "y" ? [[r.x, r.right()], [r.y, r.bottom()]] : [[r.y, r.bottom()], [r.x, r.right()]];
    if (across[0] - c < b1 && across[1] + c > b0) {
      blocked.push([along[0] - c, along[1] + c]);
    }
  };
  for (const s of ctx.boxesNear(strip, c)) {
    if (!skip.includes(s.shape.id)) {
      block(s.shape.rect);
    }
  }
  for (const l of ctx.linesNear(strip, c).map((i) => ctx.lines[i]!)) {
    if (skip.includes(l.line.id)) {
      continue;
    }
    for (const [p, q] of l.pieces) {
      block(Rect.fromPoints(p, q));
    }
  }
  for (const m of ctx.marksNear(strip, c)) {
    if (!skip.includes(m.owner)) {
      block(m.rect);
    }
  }
  return subtract(within, blocked);
}

export function subtract(within: Interval, blocked: Interval[]): Interval[] {
  blocked.sort((a, b) => totalCmp(a[0], b[0]));
  const out: Interval[] = [];
  let cur = within[0];
  for (const [lo, hi] of blocked) {
    if (cur >= within[1]) {
      break;
    }
    if (lo > cur) {
      out.push([cur, fmin(lo, within[1])]);
    }
    cur = fmax(cur, hi);
  }
  if (cur < within[1]) {
    out.push([cur, within[1]]);
  }
  return out
    .map(([lo, hi]): Interval => [round2(lo), round2(hi)])
    .filter(([lo, hi]) => hi > lo);
}

/// Free lanes parallel to a straight horizontal or vertical piece.
export class Free {
  constructor(
    public axis: Axis,
    /// Where the piece runs now.
    public at: number,
    public band: Interval,
    public intervals: Interval[],
  ) {}

  fact(): Json {
    return {
      axis: Axis.name(this.axis),
      band: [new Float(round2(this.band[0])), new Float(round2(this.band[1]))],
      intervals: this.intervals.map(([a, b]) => [new Float(a), new Float(b)]),
    };
  }

  /// e.g. "free y-intervals across x 180..520: 60..104, 176..230"
  describe(): string {
    const other = this.axis === "x" ? "y" : "x";
    const list =
      this.intervals.length === 0
        ? "none"
        : this.intervals.map(([a, b]) => `${fmtNum(a)}..${fmtNum(b)}`).join(", ");
    return `free ${Axis.name(this.axis)}-intervals across ${other} ${fmtNum(this.band[0])}..${fmtNum(this.band[1])}: ${list}`;
  }

  /// The free value closest to where the piece runs now.
  nearest(): number | null {
    let best: number | null = null;
    for (const [lo, hi] of this.intervals) {
      const v = fclamp(this.at, lo, hi);
      if (best === null || totalCmp(Math.abs(v - this.at), Math.abs(best - this.at)) < 0) {
        best = v;
      }
    }
    return best;
  }
}

/// Free intervals around the piece `pq` across the whole canvas, or `null` when `pq` is diagonal.
export function freeAround(ctx: Ctx, p: Pt, q: Pt, skip: readonly string[]): Free | null {
  const [w, h] = ctx.scene.size;
  let axis: Axis;
  let at: number;
  let band: Interval;
  let within: Interval;
  if (Math.abs(p.y - q.y) <= TOLERANCE) {
    [axis, at, band, within] = ["y", p.y, [fmin(p.x, q.x), fmax(p.x, q.x)], [0.0, h]];
  } else if (Math.abs(p.x - q.x) <= TOLERANCE) {
    [axis, at, band, within] = ["x", p.x, [fmin(p.y, q.y), fmax(p.y, q.y)], [0.0, w]];
  } else {
    return null;
  }
  return new Free(axis, at, band, freeIntervals(ctx, axis, band, within, skip));
}

/// "segment N runs at ..." help and the `free` fact, for 0-based authored segment N; or
/// "nearest free lane" and free-interval help, with the `lane` and `free` facts.
export type DeferredKind = { kind: "segment"; n: number } | { kind: "lane" };

export const DeferredKind = {
  segment(n: number): DeferredKind {
    return { kind: "segment", n };
  },
  LANE: { kind: "lane" } as DeferredKind,
};

/// Help that is costly to work out, so it is filled in only for diagnostics that are shown.
/// A `Finding` carries its entries beside its diagnostic.
export interface Deferred {
  /// Where in `help` its lines go.
  helpAt: number;
  kind: DeferredKind;
  /// The straight run the free lanes are around.
  from: [number, number];
  to: [number, number];
  /// Ids ignored when looking for free lanes.
  skip: string[];
}

/// A diagnostic and the help still to be worked out for it, which `resolve` fills in.
export class Finding {
  constructor(
    public diag: Diagnostic,
    /// In the order it was deferred.
    public deferred: Deferred[] = [],
  ) {}

  static from(f: Finding | Diagnostic): Finding {
    return f instanceof Finding ? f : new Finding(f);
  }

  withHelp(help: string): this {
    this.diag.help.push(help);
    return this;
  }

  /// The diagnostic with its deferred help worked out.
  resolved(ctx: Ctx): Diagnostic {
    resolve(ctx, this.diag, this.deferred);
    return this.diag;
  }
}

/// Defers help of `kind` around the straight run `from`..`to` to `resolve`; its lines go after
/// the help `f` has so far.
export function withDeferred(
  f0: Finding | Diagnostic,
  kind: DeferredKind,
  from: Pt,
  to: Pt,
  skip: readonly string[],
): Finding {
  const f = Finding.from(f0);
  f.deferred.push({
    helpAt: f.diag.help.length,
    kind,
    from: [from.x, from.y],
    to: [to.x, to.y],
    skip: [...skip],
  });
  return f;
}

/// Defers the `free` fact and a "segment N runs at ..." help line for `path[k]` of `l`, when it
/// is straight, to `resolve`.
export function withFree(
  f: Finding | Diagnostic,
  l: LineGeom,
  k: number,
  skip: readonly string[],
): Finding {
  const ends = l.straightEnds(k);
  if (ends === null) {
    return Finding.from(f);
  }
  return withDeferred(f, DeferredKind.segment(l.segment(k)), ends[0], ends[1], skip);
}

/// Works out the help and facts `deferred` for `d`.
export function resolve(ctx: Ctx, d: Diagnostic, deferred: readonly Deferred[]): void {
  for (const x of [...deferred].reverse()) {
    const pt = (a: [number, number]) => new Pt(a[0], a[1]);
    const f = freeAround(ctx, pt(x.from), pt(x.to), x.skip);
    if (f === null) {
      continue;
    }
    const help: string[] = [];
    if (x.kind.kind === "segment") {
      help.push(
        `segment ${x.kind.n + 1} runs at ${Axis.name(f.axis)}=${fmtNum(f.at)}; ${f.describe()}`,
      );
    } else {
      const lane = f.nearest();
      if (lane !== null) {
        d.facts.set("lane", { axis: Axis.name(f.axis), at: new Float(lane) });
        help.push(`nearest free lane: ${Axis.name(f.axis)}=${fmtNum(lane)}`);
      }
      help.push(f.describe());
    }
    d.facts.set("free", f.fact());
    const at = Math.min(x.helpAt, d.help.length);
    d.help.splice(at, 0, ...help);
  }
}

/// Free horizontal (`y`) and vertical (`x`) intervals inside `r`, for `inspect --corridors`.
export function corridors(scene: Scene, r: Rect): [Interval[], Interval[]] {
  const ctx = Ctx.new(scene);
  return [
    freeIntervals(ctx, "y", [r.x, r.right()], [r.y, r.bottom()], []),
    freeIntervals(ctx, "x", [r.y, r.bottom()], [r.x, r.right()], []),
  ];
}
