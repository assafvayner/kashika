import { code, fact } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import { Pt, TOLERANCE, distPointSeg, segHit } from "../../geom/index.js";
import { type Ctx, type LineGeom, fmtPt } from "../ctx.js";
import { arrFact, lineEl } from "../element.js";
import { type Finding, withFree } from "../free.js";
import { type Sink } from "../sink.js";

type Crossing = [number, number, Pt];

/// `cross`: two lines meet at a point that is neither a shared endpoint nor a junction.
export function cross(ctx: Ctx, out: Sink): void {
  ctx.lines.forEach((a, i) => {
    for (const j of ctx.linesNear(ctx.lineBoxes[i]!, TOLERANCE)) {
      if (j <= i) {
        continue;
      }
      const b = ctx.lines[j]!;
      for (const [ka, kb, p] of crossings(ctx, a, b)) {
        out.push(report(b, b.pieces[kb]![2], a, a.pieces[ka]![2], p));
      }
    }
  });
}

/// One point per crossing, with the indices (into `pieces`) of the pieces that meet there.
function crossings(ctx: Ctx, a: LineGeom, b: LineGeom): Crossing[] {
  const shared: [Pt, Pt][] = [];
  let hits: Crossing[] = [];
  a.pieces.forEach(([a0, a1, ai], ka) => {
    b.pieces.forEach(([b0, b1, bi], kb) => {
      const h = segHit(a0, a1, b0, b1, TOLERANCE);
      if (h === null) return;
      if (h.kind === "point") {
        hits.push([ka, kb, h.at]);
      } else if (a.straight(ai) && b.straight(bi)) {
        shared.push([h.from, h.to]);
      } else {
        hits.push([ka, kb, h.from.lerp(h.to, 0.5)]);
      }
    });
  });
  const near = (q: Pt, p: Pt) => q.dist(p) <= TOLERANCE;
  hits = hits.filter(([, , p]) => {
    const sharedEnd = (near(a.start, p) || near(a.end, p)) && (near(b.start, p) || near(b.end, p));
    return (
      !sharedEnd &&
      !ctx.isJunction(p) &&
      !shared.some(([s, e]) => distPointSeg(p, s, e) <= TOLERANCE)
    );
  });
  const clusters: Crossing[][] = [];
  for (const h of hits) {
    const joins = (c: Crossing[]) =>
      c.some(
        ([x, y, q]) =>
          q.dist(h[2]) <= 1.0 ||
          (Math.abs(x - h[0]) <= 1 &&
            Math.abs(y - h[1]) <= 1 &&
            !(a.straight(a.pieces[x]![2]) && b.straight(b.pieces[y]![2]))),
      );
    const c = clusters.find(joins);
    if (c !== undefined) {
      c.push(h);
    } else {
      clusters.push([h]);
    }
  }
  return clusters.map((c) => {
    const p = c.reduce((s, h) => s.add(h[2]), Pt.default()).mul(1.0 / c.length);
    return [c[0]![0], c[0]![1], p];
  });
}

function report(l: LineGeom, li: number, other: LineGeom, oi: number, p: Pt): Finding {
  const [id, oid] = [l.line.id, other.line.id];
  const d = Diagnostic.warn(code.CROSS, l.line.span, `${id} crosses ${oid} at ${fmtPt(p)}`)
    .withElement(lineEl(l.line, l.segment(li)))
    .withElement(lineEl(other.line, other.segment(oi)))
    .withFact(fact.AT, arrFact(p));
  const allow = other.line.named
    ? `if intentional: allow=cross:${oid} on ${id}`
    : `if intentional: name ${oid} and add allow=cross:<its id> on ${id}, or allow=cross on ${id}`;
  return withFree(d, l, li, [id]).withHelp(allow);
}
