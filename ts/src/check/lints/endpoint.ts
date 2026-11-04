import { Diagnostic } from "../../diag/diagnostic.js";
import { totalCmp } from "../../diag/rust.js";
import { type Pt, Rect, TOLERANCE, boundaryNearest, fmtNum } from "../../geom/index.js";
import { type Ctx, type ShapeGeom, anchorText, fmtPt } from "../ctx.js";
import { arrFact, lineEl, shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

/// How close a literal endpoint must be to an outline to look like a missed anchor.
const SNAP = 8.0;

/// `endpoint-off-edge`: a literal endpoint is within 8px of a shape outline but not on it.
export function endpointOffEdge(ctx: Ctx, out: Sink): void {
  for (const l of ctx.lines) {
    const ends: [Pt, boolean, string, number][] = [
      [l.start, l.line.startLiteral, "starts", 0],
      [l.end, l.line.endLiteral, "ends", l.lastSegment()],
    ];
    for (const [p, literal, verb, seg] of ends) {
      if (!literal) {
        continue;
      }
      let best: [number, Pt, ShapeGeom] | null = null;
      for (const s of ctx
        .shapesNear(new Rect(p.x, p.y, 0.0, 0.0), SNAP)
        .map((i) => ctx.shapes[i]!)
        .filter((s) => s.shape.layer === "normal")) {
        const [d, q] = boundaryNearest(p, s.poly);
        if (!(d > TOLERANCE && d <= SNAP)) continue;
        if (best === null || totalCmp(d, best[0]) < 0) best = [d, q, s];
      }
      if (best === null) continue;
      const [d, q, s] = best;
      const span =
        seg === 0 && verb === "starts" ? l.line.span : (l.line.segments[seg]?.span ?? l.line.span);
      out.push(
        Diagnostic.warn(
          "endpoint-off-edge",
          span,
          `${l.line.id} ${verb} at ${fmtPt(p)}, ${fmtNum(d)}px from the outline of '${s.shape.id}'`,
        )
          .withElement(lineEl(l.line, seg))
          .withElement(shapeEl(s.shape))
          .withFact("nearest", arrFact(q))
          .withHelp(`snap it to the outline with an anchor: \`${anchorText(s.shape, q)}\``),
      );
    }
  }
}
