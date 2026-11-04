import { Diagnostic } from "../../diag/diagnostic.js";
import { fmtNum } from "../../geom/index.js";
import { ShapeKind } from "../../model/index.js";
import { type Ctx, describeOverflow, overflow, overflowFact } from "../ctx.js";
import { bboxFact, shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

/// `text-outside-shape`: an absolute block item (`at` in the box-local frame) lies outside
/// its shape's text area, or the band under a `text-area=below` shape.
export function textOutsideShape(ctx: Ctx, out: Sink): void {
  for (const s of ctx.scene.shapes) {
    const area = s.textArea;
    for (const item of s.text.filter((t) => t.role === "absolute")) {
      const over = overflow(area, item.rect);
      if (over.length === 0) {
        continue;
      }
      const local = (x: number, y: number) =>
        `(${fmtNum(x - s.rect.x)},${fmtNum(y - s.rect.y)})`;
      out.push(
        Diagnostic.warn(
          "text-outside-shape",
          item.span,
          `block item "${item.content}" lies outside the text area of ${ShapeKind.name(s.kind)} '${s.id}' (${describeOverflow(over)})`,
        )
          .withElement(shapeEl(s))
          .withFact("bbox", bboxFact(item.rect))
          .withFact("text_area", bboxFact(area))
          .withFact("overflow", overflowFact(over))
          .withHelp(
            `keep the item within the text area, local ${local(area.x, area.y)}..${local(area.right(), area.bottom())}`,
          ),
      );
    }
  }
}
