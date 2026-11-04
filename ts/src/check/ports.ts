import { type Pt, TOLERANCE, boundaryNearest } from "../geom/index.js";
import { Marker } from "../model/index.js";
import { type Ctx, outlineAnchor } from "./ctx.js";

/// A line endpoint that sits on a shape's outline.
export interface Port {
  shape: string;
  side: string;
  t: number;
  at: Pt;
  line: string;
  /// `start` or `end`.
  end: string;
  /// Whether the line draws a marker at this end.
  marker: boolean;
  /// How far that marker reaches to either side of the line; 0 without one.
  markerHalfWidth: number;
}

/// Every endpoint on an outline, in shape order then line order. Endpoints at junctions are skipped.
export function ports(ctx: Ctx): Port[] {
  const out: Port[] = [];
  ctx.shapes.forEach((s, i) => {
    for (const l of ctx.linesNear(ctx.shapeBoxes[i]!, TOLERANCE).map((j) => ctx.lines[j]!)) {
      const ends: [Pt, string, Marker][] = [
        [l.start, "start", l.line.start],
        [l.end, "end", l.line.end],
      ];
      for (const [p, end, marker] of ends) {
        if (ctx.isJunction(p) || boundaryNearest(p, s.poly)[0] > TOLERANCE) {
          continue;
        }
        const [side, t] = outlineAnchor(s.shape, p);
        out.push({
          shape: s.shape.id,
          side,
          t,
          at: p,
          line: l.line.id,
          end,
          marker: marker !== "none",
          markerHalfWidth: Marker.halfWidth(marker, l.line.style.strokeWidth),
        });
      }
    }
  });
  return out;
}
