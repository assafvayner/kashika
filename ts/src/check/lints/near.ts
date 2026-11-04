import { code } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { fmax, fmin } from "../../diag/rust.js";
import {
  type Pt,
  TOLERANCE,
  cross,
  dot,
  edges,
  fmtNum,
  insideRuns,
  round2,
  segSegDist,
} from "../../geom/index.js";
import { ShapeKind } from "../../model/index.js";
import { type Ctx, type LineGeom, type ShapeGeom } from "../ctx.js";
import { lineEl, shapeEl } from "../element.js";
import { withFree } from "../free.js";
import { type Sink } from "../sink.js";

/// Pieces whose directions differ by less than ~3° count as parallel.
const PARALLEL = 0.05;
/// Parallel lines must run side by side for longer than this to be `near`.
const MIN_RUN = 20.0;

/// `near`: a line runs within `clearance` of a box edge, along a group border, or along a
/// parallel line for more than 20px.
export function near(ctx: Ctx, out: Sink): void {
  nearBoxes(ctx, out);
  nearGroups(ctx, out);
  nearLines(ctx, out);
}

/// Perpendicular gap and overlap length of two nearly parallel segments that overlap along their direction.
function parallel(p: Pt, q: Pt, e0: Pt, e1: Pt): [number, number] | null {
  const le = e0.dist(e1);
  if (le <= 1e-9 || p.dist(q) <= 1e-9) {
    return null;
  }
  const [u, v] = [q.sub(p).norm(), e1.sub(e0).norm()];
  if (Math.abs(cross(u, v)) > PARALLEL) {
    return null;
  }
  const [s0, s1] = [dot(p.sub(e0), v), dot(q.sub(e0), v)];
  const [lo, hi] = [fmax(fmin(s0, s1), 0.0), fmin(fmax(s0, s1), le)];
  if (hi - lo <= TOLERANCE) {
    return null;
  }
  const gap = Math.abs(cross(u, e0.add(v.mul((lo + hi) / 2.0)).sub(p)));
  return [gap, hi - lo];
}

/// How far apart two bounding boxes can be while pieces in them are within `c`: a parallel
/// gap is measured square to one piece, so allow some slack.
function reach(c: number): number {
  return 2.0 * c + 1.0;
}

function nearBoxes(ctx: Ctx, out: Sink): void {
  const c = ctx.scene.clearance;
  ctx.lines.forEach((l, i) => {
    const own = ctx.attached(l.start);
    own.push(...ctx.attached(l.end));
    for (const s of ctx.boxesNear(ctx.lineBoxes[i]!, reach(c))) {
      const id = s.shape.id;
      const attached = own.includes(id);
      if (!attached && l.pieces.some(([p, q]) => insideRuns(p, q, s.poly).length > 0)) {
        continue;
      }
      let best: [number, number, Pt, Pt] | null = null;
      for (const [p, q, k] of l.pieces) {
        for (const [e0, e1] of edges(s.poly)) {
          let gap: number | null;
          if (attached) {
            const par = parallel(p, q, e0, e1);
            gap = par === null ? null : par[0];
          } else {
            gap = segSegDist(p, q, e0, e1);
          }
          if (gap !== null && gap < c) {
            if (best === null || gap < best[0]) {
              best = [gap, k, p, q];
            }
          }
        }
      }
      if (best === null) continue;
      const [gap, k, p, q] = best;
      const elbow = attached ? wrongElbow(ctx, l, s, p, q) : null;
      if (elbow !== null) {
        const [message, help] = elbow;
        const d = Diagnostic.warn(code.NEAR, l.line.span, message)
          .withElement(lineEl(l.line, l.segment(k)))
          .withElement(shapeEl(s.shape))
          .withFact("distance", new Float(round2(gap)))
          .withFact("clearance", new Float(c))
          .withHelp(help);
        out.push(d);
        continue;
      }
      const kind = ShapeKind.name(s.shape.kind);
      const message = attached
        ? `${l.line.id} runs along the edge of its own ${kind} '${id}', ${fmtNum(gap)}px away (clearance ${fmtNum(c)})`
        : `${l.line.id} runs ${fmtNum(gap)}px from ${kind} '${id}' (clearance ${fmtNum(c)})`;
      const d = Diagnostic.warn(code.NEAR, l.line.span, message)
        .withElement(lineEl(l.line, l.segment(k)))
        .withElement(shapeEl(s.shape))
        .withFact("distance", new Float(round2(gap)))
        .withFact("clearance", new Float(c));
      const skip = [l.line.id, ...own.filter((o) => o !== id)];
      out.push(withFree(d, l, k, skip));
    }
  });
}

/// A first or last piece that runs along the side of the box the line starts or ends on: the
/// elbow is the wrong way round (`hv` into a `left` side). The message and help to fix it.
function wrongElbow(ctx: Ctx, l: LineGeom, s: ShapeGeom, p: Pt, q: Pt): [string, string] | null {
  const id = s.shape.id;
  const r = s.shape.rect;
  const vertical = Math.abs(p.x - q.x) <= TOLERANCE;
  if (!vertical && Math.abs(p.y - q.y) > TOLERANCE) {
    return null;
  }
  let side: string;
  if (vertical) {
    side = Math.abs(p.x - r.x) <= Math.abs(p.x - r.right()) ? "left" : "right";
  } else {
    side = Math.abs(p.y - r.y) <= Math.abs(p.y - r.bottom()) ? "top" : "bottom";
  }
  const on = (pt: Pt) => ctx.attached(pt).includes(id);
  let which: string;
  let help: string;
  if (q.dist(l.end) <= TOLERANCE && on(l.end)) {
    help = vertical
      ? "enter a `left`/`right` side horizontally: finish with `vh` (or an `x …` hop, then the anchor)"
      : "enter a `top`/`bottom` side vertically: finish with `hv` (or a `y …` hop, then the anchor)";
    which = "last";
  } else if (p.dist(l.start) <= TOLERANCE && on(l.start)) {
    help = vertical
      ? "leave a `left`/`right` side horizontally: start with `hv` (or an `x …` hop)"
      : "leave a `top`/`bottom` side vertically: start with `vh` (or a `y …` hop)";
    which = "first";
  } else {
    return null;
  }
  return [
    `${l.line.id}'s ${which} segment runs along the ${side} side of its own ${ShapeKind.name(s.shape.kind)} '${id}'`,
    help,
  ];
}

/// Pieces that run parallel to a group's border within `clearance`, on it included. Pieces
/// that cross the border are not parallel to it, so they don't count.
function nearGroups(ctx: Ctx, out: Sink): void {
  const c = ctx.scene.clearance;
  ctx.lines.forEach((l, i) => {
    for (const g of ctx.groupsNear(ctx.lineBoxes[i]!, reach(c))) {
      let best: [number, number] | null = null;
      for (const [p, q, k] of l.pieces) {
        for (const [e0, e1] of edges(g.poly)) {
          const par = parallel(p, q, e0, e1);
          if (par !== null && par[0] < c) {
            const gap = par[0];
            if (best === null || gap < best[0]) {
              best = [gap, k];
            }
          }
        }
      }
      if (best === null) continue;
      const [gap, k] = best;
      const id = g.shape.id;
      const message =
        gap <= TOLERANCE
          ? `${l.line.id} runs along the border of group '${id}' (clearance ${fmtNum(c)})`
          : `${l.line.id} runs ${fmtNum(gap)}px from the border of group '${id}' (clearance ${fmtNum(c)})`;
      const d = Diagnostic.warn(code.NEAR, l.line.span, message)
        .withElement(lineEl(l.line, l.segment(k)))
        .withElement(shapeEl(g.shape))
        .withFact("distance", new Float(round2(gap)))
        .withFact("clearance", new Float(c));
      out.push(withFree(d, l, k, [l.line.id]));
    }
  });
}

function nearLines(ctx: Ctx, out: Sink): void {
  const c = ctx.scene.clearance;
  ctx.lines.forEach((a, i) => {
    for (const j of ctx.linesNear(ctx.lineBoxes[i]!, reach(c))) {
      if (j <= i) {
        continue;
      }
      const b = ctx.lines[j]!;
      let run = 0.0;
      let gap = Infinity;
      let at: [number, number] | null = null;
      for (const [p, q, ai] of a.pieces) {
        let longest = 0.0;
        for (const [e0, e1, bi] of b.pieces) {
          const par = parallel(p, q, e0, e1);
          if (par === null) {
            continue;
          }
          const [g, len] = par;
          if (g > TOLERANCE && g < c) {
            longest = fmax(longest, len);
            gap = fmin(gap, g);
            at ??= [ai, bi];
          }
        }
        run += longest;
      }
      if (at === null) continue;
      const [ai, bi] = at;
      if (run <= MIN_RUN) {
        continue;
      }
      const d = Diagnostic.warn(
        code.NEAR,
        b.line.span,
        `${b.line.id} runs ${fmtNum(gap)}px from ${a.line.id} for ${fmtNum(run)}px (clearance ${fmtNum(c)})`,
      )
        .withElement(lineEl(b.line, b.segment(bi)))
        .withElement(lineEl(a.line, a.segment(ai)))
        .withFact("distance", new Float(round2(gap)))
        .withFact("length", new Float(round2(run)))
        .withFact("clearance", new Float(c));
      out.push(withFree(d, b, bi, [b.line.id]));
    }
  });
}
