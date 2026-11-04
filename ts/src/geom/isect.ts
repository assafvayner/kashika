import { fclamp, fmax, fmin, totalCmp } from "../diag/rust.js";
import { flatten, type PathSeg } from "./path.js";
import { Pt, TOLERANCE } from "./point.js";
import { type Rect } from "./rect.js";

export function cross(a: Pt, b: Pt): number {
  return a.x * b.y - a.y * b.x;
}

export function dot(a: Pt, b: Pt): number {
  return a.x * b.x + a.y * b.y;
}

/// Segments shorter than this are points.
const DEGENERATE = 1e-9;
/// Slack on the segment parameters, so endpoints that meet up to float error still hit.
const PARAM_EPS = 1e-12;

/// How two straight segments meet: at a point, or a collinear overlap from one point to
/// the other.
export type Hit = { kind: "point"; at: Pt } | { kind: "overlap"; from: Pt; to: Pt };

function hitPoint(at: Pt): Hit {
  return { kind: "point", at };
}

/// The point on segment `ab` closest to `p`.
export function closestOnSeg(p: Pt, a: Pt, b: Pt): Pt {
  const d = b.sub(a);
  const l2 = dot(d, d);
  if (l2 <= DEGENERATE * DEGENERATE) {
    return a;
  }
  return a.add(d.mul(fclamp(dot(p.sub(a), d) / l2, 0, 1)));
}

export function distPointSeg(p: Pt, a: Pt, b: Pt): number {
  return p.dist(closestOnSeg(p, a, b));
}

/// Where segments `a0a1` and `b0b1` meet. Points within `eps` of the other segment count
/// as touching, and segments within `eps` of being collinear count as overlapping.
export function segHit(a0: Pt, a1: Pt, b0: Pt, b1: Pt, eps: number): Hit | null {
  eps = fmax(eps, 0);
  const da = a1.sub(a0);
  const db = b1.sub(b0);
  const la = da.length();
  const lb = db.length();
  if (la <= DEGENERATE || lb <= DEGENERATE) {
    const [p, q0, q1] = la <= DEGENERATE ? [a0, b0, b1] : [b0, a0, a1];
    return distPointSeg(p, q0, q1) <= eps ? hitPoint(p) : null;
  }
  const denom = cross(da, db);
  if (Math.abs(denom) <= 1e-9 * la * lb) {
    if (Math.abs(cross(da, b0.sub(a0)) / la) > eps) {
      return null;
    }
    const u = da.mul(1 / la);
    const s0 = dot(b0.sub(a0), u);
    const s1 = dot(b1.sub(a0), u);
    const lo = fmax(fmin(s0, s1), 0);
    const hi = fmin(fmax(s0, s1), la);
    if (hi < lo - eps) {
      return null;
    }
    if (hi - lo <= eps) {
      return hitPoint(a0.add(u.mul(fclamp((lo + hi) / 2, 0, la))));
    }
    return { kind: "overlap", from: a0.add(u.mul(lo)), to: a0.add(u.mul(hi)) };
  }
  const t = cross(b0.sub(a0), db) / denom;
  const s = cross(b0.sub(a0), da) / denom;
  const inRange = (v: number) => v >= -PARAM_EPS && v <= 1 + PARAM_EPS;
  if (inRange(t) && inRange(s)) {
    return hitPoint(a0.add(da.mul(fclamp(t, 0, 1))));
  }
  const ends: [Pt, Pt, Pt][] = [
    [a0, b0, b1],
    [a1, b0, b1],
    [b0, a0, a1],
    [b1, a0, a1],
  ];
  const found = ends.find(([p, q0, q1]) => distPointSeg(p, q0, q1) <= eps);
  return found === undefined ? null : hitPoint(found[0]);
}

export function segSegDist(a0: Pt, a1: Pt, b0: Pt, b1: Pt): number {
  if (segHit(a0, a1, b0, b1, 0) !== null) {
    return 0;
  }
  return fmin(
    fmin(fmin(distPointSeg(a0, b0, b1), distPointSeg(a1, b0, b1)), distPointSeg(b0, a0, a1)),
    distPointSeg(b1, a0, a1),
  );
}

export function rectPolygon(r: Rect): Pt[] {
  return [
    new Pt(r.x, r.y),
    new Pt(r.right(), r.y),
    new Pt(r.right(), r.bottom()),
    new Pt(r.x, r.bottom()),
  ];
}

/// The closed polygon's edges, including the one from the last vertex back to the first.
export function edges(poly: readonly Pt[]): [Pt, Pt][] {
  const out: [Pt, Pt][] = [];
  for (let i = 0; i < poly.length; i++) {
    out.push([poly[i]!, poly[(i + 1) % poly.length]!]);
  }
  return out;
}

/// Even-odd test. Points on the boundary may land either way; use `boundaryNearest` for those.
export function pointInPolygon(p: Pt, poly: readonly Pt[]): boolean {
  let inside = false;
  for (const [a, b] of edges(poly)) {
    if (a.y > p.y !== b.y > p.y && p.x < a.x + ((p.y - a.y) * (b.x - a.x)) / (b.y - a.y)) {
      inside = !inside;
    }
  }
  return inside;
}

/// Distance from `p` to the polygon's boundary, and the nearest boundary point.
export function boundaryNearest(p: Pt, poly: readonly Pt[]): [number, Pt] {
  let best: [number, Pt] = [Infinity, p];
  for (const [a, b] of edges(poly)) {
    const q = closestOnSeg(p, a, b);
    const c: [number, Pt] = [p.dist(q), q];
    if (c[0] < best[0]) {
      best = c;
    }
  }
  return best;
}

/// Inside the polygon and farther than TOLERANCE from its boundary.
export function strictlyInside(p: Pt, poly: readonly Pt[]): boolean {
  return pointInPolygon(p, poly) && boundaryNearest(p, poly)[0] > TOLERANCE;
}

/// The parts of segment `ab` that lie strictly inside the polygon, in order from `a`.
export function insideRuns(a: Pt, b: Pt, poly: readonly Pt[]): [Pt, Pt][] {
  const d = b.sub(a);
  const l2 = dot(d, d);
  if (l2 <= DEGENERATE * DEGENERATE) {
    return [];
  }
  const param = (p: Pt) => fclamp(dot(p.sub(a), d) / l2, 0, 1);
  const ts = [0, 1];
  for (const [e0, e1] of edges(poly)) {
    const h = segHit(a, b, e0, e1, 0);
    if (h === null) continue;
    if (h.kind === "point") {
      ts.push(param(h.at));
    } else {
      ts.push(param(h.from));
      ts.push(param(h.to));
    }
  }
  ts.sort(totalCmp);
  const runs: [Pt, Pt][] = [];
  for (let i = 0; i + 1 < ts.length; i++) {
    const p = a.add(d.mul(ts[i]!));
    const q = a.add(d.mul(ts[i + 1]!));
    if (p.dist(q) <= TOLERANCE || !strictlyInside(p.lerp(q, 0.5), poly)) {
      continue;
    }
    const last = runs[runs.length - 1];
    if (last !== undefined && last[1].dist(p) <= TOLERANCE) {
      last[1] = q;
    } else {
      runs.push([p, q]);
    }
  }
  return runs;
}

/// Vertex average; inside any convex polygon.
export function centroid(poly: readonly Pt[]): Pt {
  const n = Math.max(poly.length, 1);
  return poly.reduce((s, p) => s.add(p), Pt.default()).mul(1 / n);
}

/// Whether two polygons' interiors share area. Touching edges don't count. Either polygon
/// may be non-convex.
export function polygonsOverlap(a: readonly Pt[], b: readonly Pt[]): boolean {
  if (a.length < 3 || b.length < 3 || !boundsOverlap(a, b)) {
    return false;
  }
  return (
    edges(a).some(([p, q]) => insideRuns(p, q, b).length > 0) ||
    edges(b).some(([p, q]) => insideRuns(p, q, a).length > 0) ||
    interiorSamples(a).some((p) => strictlyInside(p, b)) ||
    interiorSamples(b).some((p) => strictlyInside(p, a))
  );
}

function bounds(poly: readonly Pt[]): [Pt, Pt] {
  return poly.reduce<[Pt, Pt]>(
    ([lo, hi], p) => [
      new Pt(fmin(lo.x, p.x), fmin(lo.y, p.y)),
      new Pt(fmax(hi.x, p.x), fmax(hi.y, p.y)),
    ],
    [new Pt(Infinity, Infinity), new Pt(-Infinity, -Infinity)],
  );
}

function boundsOverlap(a: readonly Pt[], b: readonly Pt[]): boolean {
  const [alo, ahi] = bounds(a);
  const [blo, bhi] = bounds(b);
  return alo.x < bhi.x && blo.x < ahi.x && alo.y < bhi.y && blo.y < ahi.y;
}

/// Points inside the polygon: the midpoint of every interior span of a horizontal scan
/// through the middle of each band between consecutive vertex heights.
function interiorSamples(poly: readonly Pt[]): Pt[] {
  const sorted = poly.map((p) => p.y).sort(totalCmp);
  const ys: number[] = [];
  for (const y of sorted) {
    if (ys.length === 0 || ys[ys.length - 1] !== y) ys.push(y);
  }
  const out: Pt[] = [];
  for (let i = 0; i + 1 < ys.length; i++) {
    const y = (ys[i]! + ys[i + 1]!) / 2;
    const xs = edges(poly)
      .filter(([p, q]) => p.y > y !== q.y > y)
      .map(([p, q]) => p.x + ((y - p.y) * (q.x - p.x)) / (q.y - p.y))
      .sort(totalCmp);
    for (let k = 0; k + 1 < xs.length; k += 2) {
      out.push(new Pt((xs[k]! + xs[k + 1]!) / 2, y));
    }
  }
  return out;
}

/// Splits a path into straight pieces (curves flattened within FLATTEN_TOLERANCE),
/// each tagged with the index of the path segment it came from.
export function pieces(path: readonly PathSeg[]): [Pt, Pt, number][] {
  const out: [Pt, Pt, number][] = [];
  path.forEach((seg, i) => {
    const pts = flatten([seg]);
    for (let k = 0; k + 1 < pts.length; k++) {
      out.push([pts[k]!, pts[k + 1]!, i]);
    }
  });
  return out;
}
