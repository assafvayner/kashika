import { Diagnostic } from "../../diag/diagnostic.js";
import { type Rect, TOLERANCE } from "../../geom/index.js";
import { Align, type Shape, ShapeKind, type TextItem } from "../../model/index.js";
import { type Ctx } from "../ctx.js";
import { shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

/// Size of a `sublabel` relative to the label.
const SUBLABEL_SCALE = 0.8;

/// `block-reducible`: a text block of 1–2 stacked items that a `label`, optionally with a
/// `sublabel` or a second line, would draw the same way.
export function blockReducible(ctx: Ctx, out: Sink): void {
  for (const s of ctx.scene.shapes) {
    const rw = rewrite(s);
    if (rw === null) continue;
    const kind = ShapeKind.name(s.kind);
    const span = s.text[0]?.span ?? s.span;
    out.push(
      Diagnostic.warn(
        "block-reducible",
        span,
        `the text block of ${kind} '${s.id}' can be written as label/sublabel`,
      )
        .withElement(shapeEl(s))
        .withFact("rewrite", rw)
        .withHelp(`drop the block and write \`${kind} ${s.id} ${rw}\``),
    );
  }
}

/// The `"label" [sublabel="..."] [text-align=...]` text that replaces `s`'s text block.
function rewrite(s: Shape): string | null {
  if (s.text.some((t) => t.role !== "stacked")) {
    return null;
  }
  if (s.text.length !== 1 && s.text.length !== 2) {
    return null;
  }
  const first = s.text[0]!;
  const second = s.text[1] ?? null;
  const st = s.style;
  const asLabel = (t: TextItem) => t.size === st.fontSize && t.color === st.color;
  if (!asLabel(first)) {
    return null;
  }
  let text = quote(first.content);
  if (second !== null) {
    if (asLabel(second)) {
      text = quote(`${first.content}\n${second.content}`);
    } else if (
      Math.abs(second.size - st.fontSize * SUBLABEL_SCALE) < 1e-9 &&
      second.color === st.muted &&
      !second.bold
    ) {
      text = `${text} sublabel=${quote(second.content)}`;
    } else {
      return null;
    }
  }
  const a = align(s);
  if (a === null) {
    return null;
  }
  const def: Align = s.kind === "group" ? "left" : "center";
  if (a !== def) {
    text = `${text} text-align=${Align.name(a)}`;
  }
  return text;
}

/// The one alignment shared by every stacked item, preferring the shape's default.
function align(s: Shape): Align | null {
  const area = s.textArea;
  const fits = (a: Align, r: Rect) => {
    switch (a) {
      case "left":
        return same(r.x, area.x);
      case "center":
        return same(r.x + r.w / 2.0, area.x + area.w / 2.0);
      case "right":
        return same(r.right(), area.right());
    }
  };
  const order: Align[] =
    s.kind === "group" ? ["left", "center", "right"] : ["center", "left", "right"];
  return order.find((a) => s.text.every((t) => fits(a, t.rect))) ?? null;
}

function same(a: number, b: number): boolean {
  return Math.abs(a - b) <= TOLERANCE;
}

/// A Kashika string literal for text whose markup escapes are already in source form.
function quote(text: string): string {
  return `"${text.replaceAll('"', '\\"').replaceAll("\n", "\\n")}"`;
}
