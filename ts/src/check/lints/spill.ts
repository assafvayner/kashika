import { Diagnostic, type Element, type Span } from "../../diag/diagnostic.js";
import { type Pt, Rect, fmtNum, insideRuns, rectPolygon } from "../../geom/index.js";
import { ShapeKind } from "../../model/index.js";
import { type Ctx, describeOverflow, lineBbox, overflow, overflowFact } from "../ctx.js";
import { bboxFact, junctionEl, lineEl, shapeEl, textEl } from "../element.js";
import { type Sink } from "../sink.js";
import { belowBox, textBox, titleBand } from "../text_box.js";

interface Child {
  el: Element;
  span: Span;
  what: string;
  extent: Rect;
  /// Straight pieces for lines; empty for everything else.
  pieces: [Pt, Pt][];
  /// A zero-size junction: it can spill but never intrudes on the title band.
  point: boolean;
}

/// `spill`: a child extends outside its group or into the group's title band. Touching is fine.
export function spill(ctx: Ctx, out: Sink): void {
  for (const g of ctx.groups()) {
    const gid = g.shape.id;
    const band = titleBand(g.shape);
    for (const c of children(ctx, gid)) {
      const over = overflow(g.shape.rect, c.extent);
      if (over.length > 0) {
        out.push(
          Diagnostic.warn(
            "spill",
            c.span,
            `${c.what} extends outside group '${gid}' (${describeOverflow(over)})`,
          )
            .withElement(c.el)
            .withElement(shapeEl(g.shape))
            .withFact("overflow", overflowFact(over))
            .withHelp(`enlarge '${gid}' or move the child inside it`),
        );
      }
      if (band === null) continue;
      const b = band;
      let intrudes: boolean;
      if (c.point) {
        intrudes = false;
      } else if (c.pieces.length === 0) {
        intrudes = b.intersect(c.extent) !== null;
      } else {
        const poly = rectPolygon(b);
        intrudes = c.pieces.some(([p, q]) => insideRuns(p, q, poly).length > 0);
      }
      if (intrudes) {
        out.push(
          Diagnostic.warn(
            "spill",
            c.span,
            `${c.what} intrudes on the title band of group '${gid}'`,
          )
            .withElement(c.el)
            .withElement(shapeEl(g.shape))
            .withFact("band", bboxFact(b))
            .withHelp(
              g.shape.title === "top"
                ? `keep children below local y=${fmtNum(b.h)}`
                : `keep children above local y=${fmtNum(b.y - g.shape.rect.y)}`,
            ),
        );
      }
    }
  }
}

function children(ctx: Ctx, gid: string): Child[] {
  const out: Child[] = [];
  const mine = (p: string | null) => p === gid;
  for (const s of ctx.scene.shapes.filter((s) => mine(s.parent))) {
    const kind = ShapeKind.name(s.kind);
    out.push({
      el: shapeEl(s),
      span: s.span,
      what: `${kind} '${s.id}'`,
      extent: s.rect,
      pieces: [],
      point: false,
    });
    const b = belowBox(s);
    if (b !== null) {
      out.push({
        el: shapeEl(s),
        span: s.span,
        what: `the label under ${kind} '${s.id}'`,
        extent: b,
        pieces: [],
        point: false,
      });
    }
  }
  for (const j of ctx.scene.junctions.filter((j) => mine(j.parent))) {
    out.push({
      el: junctionEl(j),
      span: j.span,
      what: `junction '${j.id}'`,
      extent: new Rect(j.at.x, j.at.y, 0.0, 0.0),
      pieces: [],
      point: true,
    });
  }
  for (const t of ctx.scene.texts.filter((t) => mine(t.parent))) {
    out.push({
      el: textEl(t),
      span: t.span,
      what: `text '${t.displayId()}'`,
      extent: textBox(t),
      pieces: [],
      point: false,
    });
  }
  for (const l of ctx.lines.filter((l) => mine(l.line.parent))) {
    out.push({
      el: lineEl(l.line, null),
      span: l.line.span,
      what: `line ${l.line.id}`,
      extent: lineBbox(l),
      pieces: l.pieces.map(([p, q]): [Pt, Pt] => [p, q]),
      point: false,
    });
  }
  return out;
}
