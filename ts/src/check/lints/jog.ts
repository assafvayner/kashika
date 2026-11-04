import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { Pt, TOLERANCE, dot, fmtNum, round2 } from "../../geom/index.js";
import { type Line } from "../../model/index.js";
import { type Ctx, fmtPt } from "../ctx.js";
import { lineEl } from "../element.js";
import { type Sink } from "../sink.js";
import { anchorAt } from "./axis.js";

/// A maximal horizontal or vertical stretch of a line's unrounded path.
interface Run {
  from: Pt;
  to: Pt;
  horizontal: boolean;
  /// Index into `line.segments` of the segment that drew the start of the run.
  seg: number;
}

/// `jog`: a short horizontal or vertical step, shorter than `clearance`, between two
/// parallel runs that go the same way, so the line shows a small Z or S kink.
export function jog(ctx: Ctx, out: Sink): void {
  const limit = ctx.scene.clearance;
  for (const l of ctx.lines) {
    const rs = runs(l.line);
    const last = Math.max(rs.length - 1, 0);
    for (let i = 0; i + 3 <= rs.length; i++) {
      const [a, b, c] = [rs[i]!, rs[i + 1]!, rs[i + 2]!];
      if (a === null || b === null || c === null) {
        continue;
      }
      const step = b.from.dist(b.to);
      if (
        a.horizontal !== c.horizontal ||
        b.horizontal === a.horizontal ||
        step <= TOLERANCE ||
        step >= limit ||
        dot(a.to.sub(a.from), c.to.sub(c.from)) <= 0.0
      ) {
        continue;
      }
      const [axis, runsDir] = a.horizontal ? ["y", "horizontal"] : ["x", "vertical"];
      const value = (p: Pt) => (a.horizontal ? p.y : p.x);
      const withV = (p: Pt, v: number) => (a.horizontal ? new Pt(p.x, v) : new Pt(v, p.y));
      const span = l.line.segments[b.seg]?.span ?? l.line.span;
      const d = Diagnostic.warn(
        "jog",
        span,
        `segment ${b.seg + 1} of ${l.line.id} jogs ${fmtNum(step)}px in ${axis} between two ${runsDir} runs, from ${fmtPt(b.from)} to ${fmtPt(b.to)}`,
      )
        .withElement(lineEl(l.line, b.seg))
        .withFact("step", new Float(round2(step)))
        .withFact("clearance", new Float(round2(limit)));
      const [before, after] = [value(a.to), value(c.from)];
      const fixes: string[] = [];
      if (i + 2 === last) {
        const anc = anchorAt(ctx, c.to, withV(c.to, before));
        if (anc !== null) {
          fixes.push(`end at \`${anc}\` to keep ${axis}=${fmtNum(before)}`);
        }
      }
      if (i === 0) {
        const anc = anchorAt(ctx, a.from, withV(a.from, after));
        if (anc !== null) {
          fixes.push(`start at \`${anc}\` to keep ${axis}=${fmtNum(after)}`);
        }
      }
      const or = fixes.length === 0 ? "" : "or ";
      fixes.push(
        `${or}move the run after the step to ${axis}=${fmtNum(before)} or the run before it to ${axis}=${fmtNum(after)}, so both ${runsDir} runs share ${axis}`,
      );
      for (const f of fixes) {
        d.withHelp(f);
      }
      out.push(d);
    }
  }
}

/// The line's path before corner rounding, as horizontal or vertical runs; consecutive
/// pieces on one axis going the same way merge. Curves and diagonals are `null`.
function runs(line: Line): (Run | null)[] {
  const out: (Run | null)[] = [];
  line.segments.forEach((s, k) => {
    for (const p of s.pieces) {
      if (p.kind !== "line") {
        out.push(null);
        continue;
      }
      const { from, to } = p;
      const [dx, dy] = [Math.abs(to.x - from.x), Math.abs(to.y - from.y)];
      let horizontal: boolean;
      if (dy <= TOLERANCE && dx > dy) {
        horizontal = true;
      } else if (dx <= TOLERANCE && dy > dx) {
        horizontal = false;
      } else {
        out.push(null);
        continue;
      }
      const r = out[out.length - 1];
      if (
        r !== undefined &&
        r !== null &&
        r.horizontal === horizontal &&
        dot(r.to.sub(r.from), to.sub(from)) > 0.0
      ) {
        r.to = to;
        continue;
      }
      out.push({ from, to, horizontal, seg: k });
    }
  });
  return out;
}
