import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { fmax } from "../../diag/rust.js";
import { TOLERANCE, fmtNum, round2 } from "../../geom/index.js";
import { type Ctx, type LineGeom, fmtPt } from "../ctx.js";
import { lineEl, shapeEl } from "../element.js";
import { ports } from "../ports.js";
import { type Sink } from "../sink.js";

const MIN_GAP = 8.0;

/// `port-crowding`: two endpoints on one shape are within 8px (or closer than their two
/// markers' half-widths add up to, so crow's feet don't overlap), or are identical and both
/// draw a marker (stacked arrowheads). Junction endpoints are exempt.
export function portCrowding(ctx: Ctx, out: Sink): void {
  const all = ports(ctx);
  const byShape = new Map<string, number[]>();
  all.forEach((p, i) => {
    const v = byShape.get(p.shape);
    if (v === undefined) byShape.set(p.shape, [i]);
    else v.push(i);
  });
  all.forEach((a, i) => {
    for (const b of byShape
      .get(a.shape)!
      .filter((j) => j > i)
      .map((j) => all[j]!)) {
      const gap = a.at.dist(b.at);
      const min = fmax(MIN_GAP, a.markerHalfWidth + b.markerHalfWidth);
      const stacked = gap <= TOLERANCE;
      if (gap >= min || (stacked && !(a.marker && b.marker)) || a.line === b.line) {
        continue;
      }
      const la = ctx.line(a.line);
      const lb = ctx.line(b.line);
      const s = ctx.shape(a.shape);
      if (la === null || lb === null || s === null) {
        continue;
      }
      const message = stacked
        ? `${b.line} and ${a.line} both draw a marker at ${fmtPt(b.at)} on '${s.id}'`
        : `${b.line} and ${a.line} meet '${s.id}' ${fmtNum(gap)}px apart (minimum ${fmtNum(min)}px)`;
      out.push(
        Diagnostic.warn("port-crowding", lb.line.span, message)
          .withElement(lineEl(lb.line, endSegment(lb, b.end)))
          .withElement(lineEl(la.line, endSegment(la, a.end)))
          .withElement(shapeEl(s))
          .withFact("gap", new Float(round2(gap)))
          .withHelp(
            `spread the endpoints at least ${fmtNum(min)}px apart, e.g. \`${s.id}.${b.side}@0.3\` and \`${s.id}.${b.side}@0.7\`, or meet at a \`junction\``,
          ),
      );
    }
  });
}

function endSegment(l: LineGeom, end: string): number {
  return end === "start" ? 0 : l.lastSegment();
}
