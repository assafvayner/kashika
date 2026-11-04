import { type Json } from "../diag/json.js";
import { asUsize, fclamp, fmax, fmin, sum } from "../diag/rust.js";
import { Pt, TOLERANCE } from "./point.js";

/// Maximum distance between a curve and its flattened polyline.
export const FLATTEN_TOLERANCE = 0.25;

/// Upper bound on the pieces one curve is flattened into, whatever its size.
export const MAX_FLATTEN_STEPS = 4096;

export type PathSeg =
  | { readonly kind: "line"; readonly from: Pt; readonly to: Pt }
  | { readonly kind: "quad"; readonly from: Pt; readonly ctrl: Pt; readonly to: Pt }
  | { readonly kind: "cubic"; readonly from: Pt; readonly c1: Pt; readonly c2: Pt; readonly to: Pt };

export function line(from: Pt, to: Pt): PathSeg {
  return { kind: "line", from, to };
}

export function quad(from: Pt, ctrl: Pt, to: Pt): PathSeg {
  return { kind: "quad", from, ctrl, to };
}

export function cubic(from: Pt, c1: Pt, c2: Pt, to: Pt): PathSeg {
  return { kind: "cubic", from, c1, c2, to };
}

export function segFrom(s: PathSeg): Pt {
  return s.from;
}

export function segTo(s: PathSeg): Pt {
  return s.to;
}

/// Applies `f` to every point of the segment, control points included.
export function segMap(s: PathSeg, f: (p: Pt) => Pt): PathSeg {
  switch (s.kind) {
    case "line":
      return line(f(s.from), f(s.to));
    case "quad":
      return quad(f(s.from), f(s.ctrl), f(s.to));
    case "cubic":
      return cubic(f(s.from), f(s.c1), f(s.c2), f(s.to));
  }
}

export function segEval(s: PathSeg, t: number): Pt {
  const u = 1 - t;
  switch (s.kind) {
    case "line":
      return s.from.lerp(s.to, t);
    case "quad":
      return s.from
        .mul(u * u)
        .add(s.ctrl.mul(2 * u * t))
        .add(s.to.mul(t * t));
    case "cubic":
      return s.from
        .mul(u * u * u)
        .add(s.c1.mul(3 * u * u * t))
        .add(s.c2.mul(3 * u * t * t))
        .add(s.to.mul(t * t * t));
  }
}

export function isAxisAligned(s: PathSeg): boolean {
  switch (s.kind) {
    case "line":
      return Math.abs(s.from.x - s.to.x) <= TOLERANCE || Math.abs(s.from.y - s.to.y) <= TOLERANCE;
    default:
      return false;
  }
}

export function startDir(s: PathSeg): Pt {
  let d: Pt;
  switch (s.kind) {
    case "line":
      d = s.to.sub(s.from);
      break;
    case "quad":
      d = nonzero(s.ctrl.sub(s.from), s.to.sub(s.from));
      break;
    case "cubic":
      d = nonzero(s.c1.sub(s.from), s.to.sub(s.from));
      break;
  }
  return d.norm();
}

export function endDir(s: PathSeg): Pt {
  let d: Pt;
  switch (s.kind) {
    case "line":
      d = s.to.sub(s.from);
      break;
    case "quad":
      d = nonzero(s.to.sub(s.ctrl), s.to.sub(s.from));
      break;
    case "cubic":
      d = nonzero(s.to.sub(s.c2), s.to.sub(s.from));
      break;
  }
  return d.norm();
}

export function segLength(s: PathSeg): number {
  return length([s]);
}

/// Splits the segment at parameter `t` (de Casteljau).
export function segSplit(s: PathSeg, t: number): [PathSeg, PathSeg] {
  switch (s.kind) {
    case "line": {
      const m = s.from.lerp(s.to, t);
      return [line(s.from, m), line(m, s.to)];
    }
    case "quad": {
      const a = s.from.lerp(s.ctrl, t);
      const b = s.ctrl.lerp(s.to, t);
      const m = a.lerp(b, t);
      return [quad(s.from, a, m), quad(m, b, s.to)];
    }
    case "cubic": {
      const a = s.from.lerp(s.c1, t);
      const b = s.c1.lerp(s.c2, t);
      const c = s.c2.lerp(s.to, t);
      const d = a.lerp(b, t);
      const e = b.lerp(c, t);
      const m = d.lerp(e, t);
      return [cubic(s.from, a, d, m), cubic(m, e, c, s.to)];
    }
  }
}

/// Removes `start` and `end` px of arc length from the two ends. When they add up to
/// more than the segment's length, each end gives up length in proportion.
export function segTrim(s: PathSeg, start: number, end: number): PathSeg {
  const len = segLength(s);
  let a = fmax(start, 0);
  let b = fmax(end, 0);
  if (a + b > len) {
    const k = len / (a + b);
    a *= k;
    b *= k;
  }
  if (s.kind === "line") {
    const dir = s.to.sub(s.from).norm();
    return line(s.from.add(dir.mul(a)), s.to.sub(dir.mul(b)));
  }
  const t0 = paramAt(s, a);
  const t1 = fmax(paramAt(s, len - b), t0);
  let seg: PathSeg = s;
  if (a > 0) {
    seg = segSplit(seg, t0)[1];
  }
  if (b > 0 && t0 < 1) {
    seg = segSplit(seg, (t1 - t0) / (1 - t0))[0];
  }
  return seg;
}

/// The parameter at arc length `s` from the start, measured on the flattened curve.
function paramAt(seg: PathSeg, s: number): number {
  const n = flattenSteps(seg);
  let prev = seg.from;
  let acc = 0;
  for (let k = 1; k <= n; k++) {
    const p = segEval(seg, k / n);
    const d = prev.dist(p);
    if (acc + d >= s && d > 0) {
      return (k - 1 + (s - acc) / d) / n;
    }
    acc += d;
    prev = p;
  }
  return 1;
}

function flattenSteps(s: PathSeg): number {
  switch (s.kind) {
    case "line":
      return 1;
    case "quad":
      return steps(s.from.sub(s.ctrl.mul(2)).add(s.to).length() / 4);
    case "cubic":
      return steps(
        0.75 *
          fmax(
            s.from.sub(s.c1.mul(2)).add(s.c2).length(),
            s.c1.sub(s.c2.mul(2)).add(s.to).length(),
          ),
      );
  }
}

export function segEq(a: PathSeg, b: PathSeg): boolean {
  switch (a.kind) {
    case "line":
      return b.kind === "line" && a.from.eq(b.from) && a.to.eq(b.to);
    case "quad":
      return b.kind === "quad" && a.from.eq(b.from) && a.ctrl.eq(b.ctrl) && a.to.eq(b.to);
    case "cubic":
      return (
        b.kind === "cubic" &&
        a.from.eq(b.from) &&
        a.c1.eq(b.c1) &&
        a.c2.eq(b.c2) &&
        a.to.eq(b.to)
      );
  }
}

export function segToJson(s: PathSeg): Json {
  switch (s.kind) {
    case "line":
      return { kind: "line", from: s.from.toJson(), to: s.to.toJson() };
    case "quad":
      return { kind: "quad", from: s.from.toJson(), ctrl: s.ctrl.toJson(), to: s.to.toJson() };
    case "cubic":
      return {
        kind: "cubic",
        from: s.from.toJson(),
        c1: s.c1.toJson(),
        c2: s.c2.toJson(),
        to: s.to.toJson(),
      };
  }
}

function nonzero(a: Pt, fallback: Pt): Pt {
  return a.length() > 1e-9 ? a : fallback;
}

/// Uniform step count that keeps the chord error of a curve with the given
/// second-derivative bound under FLATTEN_TOLERANCE.
function steps(deviation: number): number {
  return Math.min(
    Math.max(asUsize(Math.ceil(Math.sqrt(deviation / FLATTEN_TOLERANCE))), 1),
    MAX_FLATTEN_STEPS,
  );
}

export function flatten(path: readonly PathSeg[]): Pt[] {
  const pts: Pt[] = [];
  for (const seg of path) {
    if (pts.length === 0) {
      pts.push(seg.from);
    }
    const n = flattenSteps(seg);
    for (let k = 1; k <= n; k++) {
      pts.push(segEval(seg, k / n));
    }
  }
  return pts;
}

export function length(path: readonly PathSeg[]): number {
  const pts = flatten(path);
  return sum(windowDists(pts));
}

function* windowDists(pts: readonly Pt[]): Generator<number> {
  for (let i = 0; i + 1 < pts.length; i++) {
    yield pts[i]!.dist(pts[i + 1]!);
  }
}

export function pointAt(path: readonly PathSeg[], t: number): Pt {
  const pts = flatten(path);
  const total = sum(windowDists(pts));
  let remaining = total * fclamp(t, 0, 1);
  for (let i = 0; i + 1 < pts.length; i++) {
    const w0 = pts[i]!;
    const w1 = pts[i + 1]!;
    const d = w0.dist(w1);
    if (remaining <= d && d > 0) {
      return w0.lerp(w1, remaining / d);
    }
    remaining -= d;
  }
  return pts.length > 0 ? pts[pts.length - 1]! : Pt.default();
}

/// Rounds the corners between consecutive straight segments; curves pass through unchanged.
export function roundCorners(path: readonly PathSeg[], r: number): PathSeg[] {
  const tagged: [PathSeg, null][] = path.map((s) => [s, null]);
  return roundCornersTagged(tagged, r).map(([s]) => s);
}

/// Like `roundCorners`, carrying each input segment's tag to the pieces made from it.
/// A corner arc takes the tag of the segment that leads into it.
export function roundCornersTagged<T>(
  path: readonly (readonly [PathSeg, T])[],
  r: number,
): [PathSeg, T][] {
  const out: [PathSeg, T][] = [];
  const run: [Pt, T][] = [];
  for (const [seg, tag] of path) {
    if (seg.kind === "line") {
      if (run.length === 0) {
        run.push([seg.from, tag]);
      }
      run.push([seg.to, tag]);
    } else {
      flushRun(run, r, out);
      out.push([seg, tag]);
    }
  }
  flushRun(run, r, out);
  return out;
}

/// `run[0]` is the start point; `run[k]` is the end of straight segment `k-1` with its tag.
function flushRun<T>(run: [Pt, T][], r: number, out: [PathSeg, T][]): void {
  if (run.length < 2) {
    run.length = 0;
    return;
  }
  let cursor = run[0]![0];
  for (let i = 0; i + 2 < run.length; i++) {
    const prev = run[i]![0];
    const [corner, tag] = run[i + 1]!;
    const next = run[i + 2]![0];
    const re = fmin(fmin(r, prev.dist(corner) / 2), corner.dist(next) / 2);
    if (re <= 1e-9) {
      if (cursor.dist(corner) > 1e-9) {
        out.push([line(cursor, corner), tag]);
      }
      cursor = corner;
      continue;
    }
    const a = corner.add(prev.sub(corner).norm().mul(re));
    const b = corner.add(next.sub(corner).norm().mul(re));
    if (cursor.dist(a) > 1e-9) {
      out.push([line(cursor, a), tag]);
    }
    out.push([quad(a, corner, b), tag]);
    cursor = b;
  }
  const [last, tag] = run[run.length - 1]!;
  if (cursor.dist(last) > 1e-9) {
    out.push([line(cursor, last), tag]);
  }
  run.length = 0;
}
