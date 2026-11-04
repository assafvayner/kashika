import { code, fact } from "../../diag/codes.js";
import { Diagnostic } from "../../diag/diagnostic.js";
import { type Pt, type Rect, TOLERANCE, insideRuns, strictlyInside } from "../../geom/index.js";
import { ShapeKind } from "../../model/index.js";
import { type Ctx } from "../ctx.js";
import { arrFact, lineEl, shapeEl } from "../element.js";
import { withFree } from "../free.js";
import { type Sink } from "../sink.js";

/// `through`: a line enters a box's interior. For the boxes a line starts and ends at, only the
/// run from an endpoint strictly inside the box out to its outline is exempt: reaching an
/// endpoint on the outline from inside the box is flagged as running through its own endpoint
/// box, and leaving and re-entering as passing through it.
export function through(ctx: Ctx, out: Sink): void {
  ctx.lines.forEach((l, i) => {
    const [starts, ends] = [ctx.attached(l.start), ctx.attached(l.end)];
    for (const s of ctx.boxesNear(ctx.lineBoxes[i]!, TOLERANCE)) {
      const id = s.shape.id;
      let runs: [Pt, Pt, number][] = [];
      for (const [p, q, k] of l.pieces) {
        for (const [a, b] of insideRuns(p, q, s.poly)) {
          const last = runs[runs.length - 1];
          if (last !== undefined && last[1].dist(a) <= TOLERANCE) {
            last[1] = b;
          } else {
            runs.push([a, b, k]);
          }
        }
      }
      const [isStart, isEnd] = [starts.includes(id), ends.includes(id)];
      if (isStart && strictlyInside(l.start, s.poly)) {
        runs = runs.filter((r) => r[0].dist(l.start) > TOLERANCE);
      }
      if (isEnd && strictlyInside(l.end, s.poly)) {
        runs = runs.filter((r) => r[1].dist(l.end) > TOLERANCE);
      }
      const lastRun = runs[runs.length - 1];
      const firstRun = runs[0];
      const atEnd = isEnd && lastRun !== undefined && lastRun[1].dist(l.end) <= TOLERANCE;
      const atStart = isStart && firstRun !== undefined && firstRun[0].dist(l.start) <= TOLERANCE;
      const kind = ShapeKind.name(s.shape.kind);
      if (atEnd || atStart) {
        const [entry, exit, k] = atEnd ? lastRun! : firstRun!;
        const help = atEnd
          ? `arrive at '${id}' from outside: keep the last corner beyond the side it ends on, so a \`left\`/\`right\` end is reached by a horizontal run from outside (\`vh\`, or an \`x …\` hop first) and a \`top\`/\`bottom\` end by a vertical one (\`hv\`, \`y …\`)`
          : `leave '${id}' straight out of the side it starts on: from \`left\`/\`right\` run horizontally clear of it first (\`hv\`, or an \`x …\` hop), from \`top\`/\`bottom\` vertically (\`vh\`, \`y …\`)`;
        out.push(
          Diagnostic.warn(
            code.THROUGH,
            l.line.span,
            `${l.line.id} runs through the inside of its own endpoint ${kind} '${id}'`,
          )
            .withElement(lineEl(l.line, l.segment(k)))
            .withElement(shapeEl(s.shape))
            .withFact(fact.ENTRY, arrFact(entry))
            .withFact(fact.EXIT, arrFact(exit))
            .withHelp(help),
        );
        continue;
      }
      if (firstRun === undefined || lastRun === undefined) {
        continue;
      }
      const [entry, , k] = firstRun;
      const exit = lastRun[1];
      const skip = [l.line.id, ...starts, ...ends];
      const d = Diagnostic.warn(
        code.THROUGH,
        l.line.span,
        `${l.line.id} passes through ${kind} '${id}'`,
      )
        .withElement(lineEl(l.line, l.segment(k)))
        .withElement(shapeEl(s.shape))
        .withFact(fact.ENTRY, arrFact(entry))
        .withFact(fact.EXIT, arrFact(exit));
      let f = withFree(d, l, k, skip);
      const stop =
        runs.length === 1 && !isStart && !isEnd
          ? stopSides(s.shape.rect, runs[0]![0], runs[0]![1])
          : null;
      if (stop !== null) {
        const [into, from] = stop;
        f = f.withHelp(
          `if '${id}' is a stop on this path, split the line at it: one line ending at '${id}'.${into}, another starting at '${id}'.${from}`,
        );
      }
      out.push(f);
    }
  });
}

/// The sides a straight run from `entry` to `exit` goes in and out of `r` by, when it crosses
/// between opposite sides near the middle, as a line through a stop on its path would.
function stopSides(r: Rect, entry: Pt, exit: Pt): [string, string] | null {
  const c = r.center();
  let d: number;
  let off: number;
  let across: number;
  let along: number;
  let sides: [string, string];
  if (Math.abs(entry.y - exit.y) <= TOLERANCE) {
    [d, off, across, along, sides] = [exit.x - entry.x, entry.y - c.y, r.h, r.w, ["left", "right"]];
  } else if (Math.abs(entry.x - exit.x) <= TOLERANCE) {
    [d, off, across, along, sides] = [exit.y - entry.y, entry.x - c.x, r.w, r.h, ["top", "bottom"]];
  } else {
    return null;
  }
  if (Math.abs(off) > across / 4.0 || Math.abs(d) < along / 2.0) {
    return null;
  }
  return d > 0.0 ? sides : [sides[1], sides[0]];
}
