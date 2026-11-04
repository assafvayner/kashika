import { code, fact } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import {
  type Pt,
  Rect,
  TOLERANCE,
  boundaryNearest,
  distPointSeg,
  insideRuns,
  pointInPolygon,
} from "../../geom/index.js";
import { type Ctx, type LineGeom } from "../ctx.js";
import { arrFact, lineEl, shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

/// `group-transit`: a line passes through a group's interior while neither endpoint is inside
/// it, and no other line that meets it at a junction has an endpoint inside it either.
export function groupTransit(ctx: Ctx, out: Sink): void {
  ctx.lines.forEach((l, i) => {
    for (const g of ctx.groupsNear(ctx.lineBoxes[i]!, TOLERANCE)) {
      const inside = (p: Pt) =>
        pointInPolygon(p, g.poly) || boundaryNearest(p, g.poly)[0] <= TOLERANCE;
      if (inside(l.start) || inside(l.end)) {
        continue;
      }
      let hit: [Pt, number] | null = null;
      for (const [p, q, k] of l.pieces) {
        const r = insideRuns(p, q, g.poly)[0];
        if (r !== undefined) {
          hit = [r[0], k];
          break;
        }
      }
      if (hit === null) continue;
      const [entry, k] = hit;
      if (taps(ctx, i).some((t) => inside(t.start) || inside(t.end))) {
        continue;
      }
      out.push(
        Diagnostic.warn(
          code.GROUP_TRANSIT,
          l.line.span,
          `${l.line.id} passes through group '${g.shape.id}' without connecting to anything inside it`,
        )
          .withElement(lineEl(l.line, l.segment(k)))
          .withElement(shapeEl(g.shape))
          .withFact(fact.ENTRY, arrFact(entry))
          .withHelp("route around the group, for example along its outside margin"),
      );
    }
  });
}

/// The other lines with an endpoint at a junction that lies on line `i`.
function taps(ctx: Ctx, i: number): LineGeom[] {
  const l = ctx.lines[i]!;
  return ctx
    .junctionsNear(ctx.lineBoxes[i]!, TOLERANCE)
    .filter((j) => l.pieces.some(([p, q]) => distPointSeg(j.at, p, q) <= TOLERANCE))
    .flatMap((j) => {
      const at = new Rect(j.at.x, j.at.y, 0.0, 0.0);
      return ctx
        .linesNear(at, TOLERANCE)
        .filter((t) => t !== i)
        .map((t) => ctx.lines[t]!)
        .filter((t) => t.start.dist(j.at) <= TOLERANCE || t.end.dist(j.at) <= TOLERANCE);
    });
}
