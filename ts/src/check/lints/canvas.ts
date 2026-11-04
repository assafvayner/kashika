import { Diagnostic, type Element, type Span } from "../../diag/diagnostic.js";
import { Rect, fmtNum } from "../../geom/index.js";
import { type Shape, ShapeKind } from "../../model/index.js";
import { type Ctx, describeOverflow, lineBbox, overflow, overflowFact } from "../ctx.js";
import { junctionEl, lineEl, shapeEl, textEl } from "../element.js";
import { type Sink } from "../sink.js";
import { belowBox, labelBox, textBox } from "../text_box.js";

/// `out-of-canvas`: an element extends beyond the diagram size. Strokes and markers are ignored.
export function outOfCanvas(ctx: Ctx, out: Sink): void {
  const [w, h] = ctx.scene.size;
  if (w <= 0.0 || h <= 0.0) {
    return;
  }
  const canvas = new Rect(0.0, 0.0, w, h);
  const fitted = ctx.scene.fitted;
  const check = (what: string, el: Element, span: Span, extent: Rect) => {
    const over = overflow(canvas, extent);
    if (over.length === 0) {
      return;
    }
    const d = Diagnostic.warn(
      "out-of-canvas",
      span,
      `${what} extends beyond the ${fmtNum(w)}x${fmtNum(h)} canvas (${describeOverflow(over)})`,
    )
      .withElement(el)
      .withFact("overflow", overflowFact(over));
    const shifts: string[] = [];
    for (const [side, by] of over) {
      if (side === "left") shifts.push(`right by ${fmtNum(by)}px`);
      else if (side === "top") shifts.push(`down by ${fmtNum(by)}px`);
    }
    if (shifts.length > 0) {
      const f = fitted[0] || fitted[1] ? "; a fitted canvas grows only right and down" : "";
      d.withHelp(
        `move it ${shifts.join(" and ")} (or everything, to keep the layout): the canvas starts at (0,0)${f}`,
      );
    }
    if (shifts.length < over.length) {
      d.withHelp(
        `move it inside (0,0)-(${fmtNum(w)},${fmtNum(h)}) or enlarge \`size\` on the \`diagram\` line`,
      );
    }
    out.push(d);
  };
  const escapes = (s: Shape) => overflow(canvas, s.rect).length > 0;
  const insideEscapingGroup = (s: Shape) => {
    let parent = s.parent;
    while (parent !== null) {
      const id = parent;
      const p = ctx.scene.shapes.find((x) => x.id === id);
      if (p === undefined) {
        return false;
      }
      if (escapes(p)) {
        return true;
      }
      parent = p.parent;
    }
    return false;
  };
  for (const s of ctx.scene.shapes) {
    if (insideEscapingGroup(s)) {
      continue;
    }
    const kind = ShapeKind.name(s.kind);
    check(`${kind} '${s.id}'`, shapeEl(s), s.span, s.rect);
    const b = belowBox(s);
    if (b !== null) {
      check(`the label under ${kind} '${s.id}'`, shapeEl(s), s.span, b);
    }
  }
  for (const j of ctx.scene.junctions) {
    check(`junction '${j.id}'`, junctionEl(j), j.span, new Rect(j.at.x, j.at.y, 0.0, 0.0));
  }
  for (const t of ctx.scene.texts) {
    check(`text '${t.displayId()}'`, textEl(t), t.span, textBox(t));
  }
  for (const l of ctx.lines) {
    check(`line ${l.line.id}`, lineEl(l.line, null), l.line.span, lineBbox(l));
    for (const lab of l.line.labels) {
      check(`label "${lab.text}" of ${l.line.id}`, lineEl(l.line, null), lab.span, labelBox(lab));
    }
  }
}
