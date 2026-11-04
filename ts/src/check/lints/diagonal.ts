import { Diagnostic } from "../../diag/diagnostic.js";
import { isAxisAligned, segFrom, segTo } from "../../geom/index.js";
import { type Ctx, fmtPt } from "../ctx.js";
import { lineEl } from "../element.js";
import { type Sink } from "../sink.js";
import { slant } from "./axis.js";

/// `diagonal-tail`: the final segment is diagonal while the line has at least one other
/// straight segment and all of those are horizontal or vertical. A line whose only straight
/// segment is the diagonal has no tail. A tail within 5° of an axis is `near-axis` instead.
export function diagonalTail(ctx: Ctx, out: Sink): void {
  for (const l of ctx.lines) {
    const path = l.line.path;
    const straight: number[] = [];
    path.forEach((s, i) => {
      if (s.kind === "line") straight.push(i);
    });
    const last = straight[straight.length - 1];
    if (last === undefined) {
      continue;
    }
    const tail = path[last]!;
    if (
      last !== path.length - 1 ||
      straight.length < 2 ||
      isAxisAligned(tail) ||
      slant(segFrom(tail), segTo(tail)) !== null ||
      !straight.slice(0, -1).every((i) => isAxisAligned(path[i]!))
    ) {
      continue;
    }
    const seg = l.segment(last);
    const span = l.line.segments[seg]?.span ?? l.line.span;
    out.push(
      Diagnostic.warn(
        "diagonal-tail",
        span,
        `the last segment of ${l.line.id} runs diagonally from ${fmtPt(segFrom(tail))} to ${fmtPt(segTo(tail))}`,
      )
        .withElement(lineEl(l.line, seg))
        .withHelp(
          "finish with `hv`/`vh` or an `x`/`y` target so the last hop is horizontal or vertical",
        ),
    );
  }
}
