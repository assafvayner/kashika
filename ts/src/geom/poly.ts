import { fmax, fmin, totalCmp } from "../diag/rust.js";
import { type Pt } from "./point.js";
import { type Rect } from "./rect.js";

/// Convex hull (Andrew's monotone chain), counter-clockwise in y-up terms, without
/// collinear points. Fewer than three distinct points come back as they are, sorted.
export function convexHull(pts: readonly Pt[]): Pt[] {
  const sorted = pts
    .filter((q) => Number.isFinite(q.x) && Number.isFinite(q.y))
    .sort((a, b) => totalCmp(a.x, b.x) || totalCmp(a.y, b.y));
  const p: Pt[] = [];
  for (const q of sorted) {
    if (p.length === 0 || !p[p.length - 1]!.eq(q)) p.push(q);
  }
  if (p.length < 3) {
    return p;
  }
  const cross = (o: Pt, a: Pt, b: Pt) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
  const lower: Pt[] = [];
  for (const q of p) {
    while (lower.length >= 2 && cross(lower[lower.length - 2]!, lower[lower.length - 1]!, q) <= 0) {
      lower.pop();
    }
    lower.push(q);
  }
  const upper: Pt[] = [];
  for (let i = p.length - 1; i >= 0; i--) {
    const q = p[i]!;
    while (upper.length >= 2 && cross(upper[upper.length - 2]!, upper[upper.length - 1]!, q) <= 0) {
      upper.pop();
    }
    upper.push(q);
  }
  lower.pop();
  upper.pop();
  return lower.concat(upper);
}

/// The y values where the vertical line `x` meets the edges of the closed polygon `poly`.
export function verticalHits(poly: readonly Pt[], x: number): number[] {
  return hits(poly, x, (p) => [p.x, p.y]);
}

/// The x values where the horizontal line `y` meets the edges of the closed polygon `poly`.
export function horizontalHits(poly: readonly Pt[], y: number): number[] {
  return hits(poly, y, (p) => [p.y, p.x]);
}

/// Sorted, with hits closer than 1e-9 merged, so a cut through a vertex reports it once.
/// `axes` returns (coordinate along the cut, coordinate reported) for a point.
function hits(poly: readonly Pt[], at: number, axes: (p: Pt) => [number, number]): number[] {
  const out: number[] = [];
  for (let i = 0; i < poly.length; i++) {
    const [a0, a1] = axes(poly[i]!);
    const [b0, b1] = axes(poly[(i + 1) % poly.length]!);
    if (at < fmin(a0, b0) - 1e-9 || at > fmax(a0, b0) + 1e-9) {
      continue;
    }
    if (Math.abs(b0 - a0) < 1e-12) {
      out.push(a1);
      out.push(b1);
    } else {
      out.push(a1 + ((b1 - a1) * (at - a0)) / (b0 - a0));
    }
  }
  out.sort(totalCmp);
  const merged: number[] = [];
  for (const v of out) {
    if (merged.length === 0 || !(Math.abs(v - merged[merged.length - 1]!) < 1e-9)) merged.push(v);
  }
  return merged;
}

/// The part of the convex polygon `poly` inside `r` (Sutherland-Hodgman). Empty when they
/// do not meet.
export function clipToRect(poly: readonly Pt[], r: Rect): Pt[] {
  const coord = (p: Pt, x: boolean) => (x ? p.x : p.y);
  const planes: [boolean, number, number][] = [
    [true, r.x, 1],
    [true, r.right(), -1],
    [false, r.y, 1],
    [false, r.bottom(), -1],
  ];
  let out = poly.slice();
  for (const [x, at, sign] of planes) {
    const inside = (p: Pt) => (coord(p, x) - at) * sign >= 0;
    const input = out;
    out = [];
    input.forEach((b, i) => {
      const a = input[(i + input.length - 1) % input.length]!;
      if (inside(b) !== inside(a)) {
        const t = (at - coord(a, x)) / (coord(b, x) - coord(a, x));
        out.push(a.lerp(b, t));
      }
      if (inside(b)) {
        out.push(b);
      }
    });
  }
  return out;
}
