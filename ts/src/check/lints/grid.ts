import { Diagnostic } from "../../diag/diagnostic.js";
import { Float } from "../../diag/json.js";
import { round } from "../../diag/rust.js";
import { fmtNum } from "../../geom/index.js";
import { type Ctx } from "../ctx.js";
import { type Sink } from "../sink.js";

/// `off-grid`: an authored literal is not a multiple of `grid`. Resolved anchors are never checked.
export function offGrid(ctx: Ctx, out: Sink): void {
  const g = ctx.scene.grid;
  if (g === null || !(g > 0.0)) {
    return;
  }
  for (const l of ctx.scene.literals) {
    const r = l.value / g;
    if (!(Math.abs(r - round(r)) * g > 1e-6)) {
      continue;
    }
    const lo = Math.floor(l.value / g) * g;
    const d = Diagnostic.warn(
      "off-grid",
      l.span,
      `${l.what}=${fmtNum(l.value)} on '${l.owner}' is not a multiple of grid ${fmtNum(g)}`,
    )
      .withFact("value", new Float(l.value))
      .withFact("grid", new Float(g))
      .withHelp(`use ${fmtNum(lo)} or ${fmtNum(lo + g)}`);
    const e = ctx.element(l.owner);
    if (e !== null) {
      d.withElement(e);
    }
    out.push(d);
  }
}
