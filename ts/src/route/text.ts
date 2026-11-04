import { Pt, fmtNum as num } from "../geom/index.js";
import { type Hop, type Lane, type Plan } from "./candidates.js";
import { type Endpoint } from "./endpoint.js";

function point(at: Pt, expr: string | null, origin: Pt): string {
  if (expr !== null) {
    return expr;
  }
  return `(${num(at.x - origin.x)},${num(at.y - origin.y)})`;
}

/// The path part of a `line` statement. Literals are written relative to `origin`, the
/// top-left of the group the line belongs to. A literal lane halfway to the end followed by
/// the elbow to the end is written as one `hvh` or `vhv`.
export function pathText(from: Endpoint, to: Endpoint, plan: Plan, origin: Pt): string {
  const end = point(to.at, to.expr, origin);
  let s = point(from.at, from.expr, origin);
  let at = from.at;
  const halfway = (l: Lane, a: number, b: number) =>
    l.expr === null && a !== b && l.value === (a + b) / 2.0;
  const hops = plan.hops;
  for (let i = 0; i < hops.length; i++) {
    const hop: Hop = hops[i]!;
    const next = hops[i + 1];
    let h: string;
    if (hop.kind === "X" && next?.kind === "Vh" && halfway(hop.value, at.x, to.at.x)) {
      i++;
      h = `hvh ${end}`;
    } else if (hop.kind === "Y" && next?.kind === "Hv" && halfway(hop.value, at.y, to.at.y)) {
      i++;
      h = `vhv ${end}`;
    } else {
      switch (hop.kind) {
        case "X":
          at = new Pt(hop.value.value, at.y);
          h = `x ${hop.value.expr ?? num(hop.value.value - origin.x)}`;
          break;
        case "Y":
          at = new Pt(at.x, hop.value.value);
          h = `y ${hop.value.expr ?? num(hop.value.value - origin.y)}`;
          break;
        case "To":
          h = end;
          break;
        case "Hv":
          h = `hv ${end}`;
          break;
        case "Vh":
          h = `vh ${end}`;
          break;
        case "Curve":
          h = `curve ${hop.left ? "left" : "right"} ${num(hop.amount)} ${end}`;
          break;
        case "Via":
          h = `curve via ${point(hop.value[0].at, hop.value[0].expr, origin)} ${point(hop.value[1].at, hop.value[1].expr, origin)} ${end}`;
          break;
      }
    }
    s += " -> ";
    s += h;
  }
  return s;
}

/// A `line` statement, or an `arrow` one when `arrow` is set.
export function statement(arrow: boolean, id: string, path: string, attrs: string): string {
  const kw = arrow ? "arrow" : "line";
  if (attrs === "") {
    return `${kw} ${id} ${path}`;
  } else {
    return `${kw} ${id} ${path} ${attrs}`;
  }
}
