import { Diagnostic } from "../../diag/diagnostic.js";
import { TOLERANCE, polygonsOverlap } from "../../geom/index.js";
import { type Shape, ShapeKind } from "../../model/index.js";
import { type Ctx } from "../ctx.js";
import { bboxFact, shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

/// `overlap`: two shapes have intersecting interiors and neither is a group that contains the other.
/// Shapes nested in the source (ancestor and descendant) are skipped; `spill` covers those.
export function overlap(ctx: Ctx, out: Sink): void {
  ctx.shapes.forEach((a, i) => {
    for (const j of ctx.shapesNear(ctx.shapeBoxes[i]!, TOLERANCE)) {
      if (j <= i) {
        continue;
      }
      const b = ctx.shapes[j]!;
      const [sa, sb] = [a.shape, b.shape];
      if (
        sa.layer !== "normal" ||
        sb.layer !== "normal" ||
        ctx.isAncestor(sa.id, sb.id) ||
        ctx.isAncestor(sb.id, sa.id)
      ) {
        continue;
      }
      const groupHolds = (outer: Shape, inner: Shape) =>
        outer.kind === "group" && outer.rect.containsRect(inner.rect);
      if (groupHolds(sa, sb) || groupHolds(sb, sa) || !polygonsOverlap(a.poly, b.poly)) {
        continue;
      }
      const d = Diagnostic.warn(
        "overlap",
        sb.span,
        `${ShapeKind.name(sb.kind)} '${sb.id}' overlaps ${ShapeKind.name(sa.kind)} '${sa.id}'`,
      )
        .withElement(shapeEl(sb))
        .withElement(shapeEl(sa));
      const r = sa.rect.intersect(sb.rect);
      if (r !== null) {
        d.withFact("intersection", bboxFact(r));
      }
      if (sa.rect.containsRect(sb.rect) || sb.rect.containsRect(sa.rect)) {
        d.withHelp(
          "only a group may contain another shape; make the outer one a `group` or move the inner one out",
        );
      }
      out.push(d);
    }
  });
}
