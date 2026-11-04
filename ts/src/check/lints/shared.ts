import { code } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { type Pt, TOLERANCE, fmtNum, round2, segHit } from "../../geom/index.js";
import { type Ctx, type LineGeom, fmtPt } from "../ctx.js";
import { arrFact, lineEl } from "../element.js";
import { DeferredKind, type Finding, withDeferred } from "../free.js";
import { type Sink } from "../sink.js";

/// `shared-segment`: two lines overlap collinearly for more than 1px. Only straight pieces
/// are compared, because flattened curves would report artifacts.
export function sharedSegment(ctx: Ctx, out: Sink): void {
  ctx.lines.forEach((a, i) => {
    for (const j of ctx.linesNear(ctx.lineBoxes[i]!, TOLERANCE)) {
      if (j <= i) {
        continue;
      }
      const b = ctx.lines[j]!;
      let total = 0.0;
      let first: [Pt, Pt, number, number] | null = null;
      for (const [a0, a1, ai] of a.pieces.filter((p) => a.straight(p[2]))) {
        for (const [b0, b1, bi] of b.pieces.filter((p) => b.straight(p[2]))) {
          const h = segHit(a0, a1, b0, b1, TOLERANCE);
          if (h !== null && h.kind === "overlap") {
            const [p, q] = [h.from, h.to];
            if (p.dist(q) > 1.0) {
              total += p.dist(q);
              first ??= [p, q, ai, bi];
            }
          }
        }
      }
      if (first !== null) {
        const [p, q, ai, bi] = first;
        out.push(report(b, bi, a, ai, [p, q], total));
      }
    }
  });
}

function report(
  l: LineGeom,
  li: number,
  other: LineGeom,
  oi: number,
  [p, q]: [Pt, Pt],
  total: number,
): Finding {
  const id = l.line.id;
  const d = Diagnostic.warn(
    code.SHARED_SEGMENT,
    l.line.span,
    `${id} runs along ${other.line.id} for ${fmtNum(total)}px, from ${fmtPt(p)} to ${fmtPt(q)}`,
  )
    .withElement(lineEl(l.line, l.segment(li)))
    .withElement(lineEl(other.line, other.segment(oi)))
    .withFact("from", arrFact(p))
    .withFact("to", arrFact(q))
    .withFact("length", new Float(round2(total)));
  const f = withDeferred(d, DeferredKind.LANE, p, q, [id]);
  const sharedEnd = (
    [
      [l.start, other.start],
      [l.end, other.end],
    ] as [Pt, Pt][]
  ).some(([a, b]) => a.dist(b) <= TOLERANCE);
  if (sharedEnd) {
    return f.withHelp(
      "both lines leave or reach the same point: spread them along the side with `side@t` (`a.right@0.3`, `a.right@0.7`), or fan out from a `junction`",
    );
  }
  return f;
}
