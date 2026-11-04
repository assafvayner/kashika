import { Diagnostic, type Element, type Span } from "../../diag/diagnostic.js";
import { type PlacedLine } from "../../model/index.js";
import { debugStr } from "../../syntax/index.js";
import { coveredScripts, missingGlyphs } from "../../text/index.js";
import { type Ctx } from "../ctx.js";
import { lineEl, shapeEl, textEl } from "../element.js";
import { type Sink } from "../sink.js";

/// `missing-glyph`: text uses characters the bundled fonts cannot draw. They render as the
/// font's empty-box `.notdef` glyph.
export function missingGlyph(ctx: Ctx, out: Sink): void {
  for (const s of ctx.shapes) {
    for (const item of s.shape.text) {
      check(out, item.lines, item.span, `text of '${s.shape.id}'`, shapeEl(s.shape));
    }
  }
  for (const t of ctx.scene.texts) {
    check(out, t.lines, t.span, `text '${t.displayId()}'`, textEl(t));
  }
  for (const l of ctx.lines) {
    for (const lab of l.line.labels) {
      check(out, lab.lines, lab.span, `label of ${l.line.id}`, lineEl(l.line, null));
    }
  }
}

/// Rust's `char::escape_debug`.
function escapeDebugChar(c: string): string {
  return c === "'" ? "\\'" : debugStr(c).slice(1, -1);
}

function check(
  out: Sink,
  lines: readonly PlacedLine[],
  span: Span,
  what: string,
  el: Element,
): void {
  const missing = missingGlyphs(lines.flatMap((l) => l.runs));
  if (missing.length === 0) {
    return;
  }
  const list = missing.map(
    (c) => `U+${c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")}`,
  );
  out.push(
    Diagnostic.warn(
      "missing-glyph",
      span,
      `${what} uses ${missing.length === 1 ? "a character" : "characters"} the bundled fonts cannot draw: ${missing.map((c) => `'${escapeDebugChar(c)}'`).join(", ")}`,
    )
      .withElement(el)
      .withFact("codepoints", list)
      .withHelp(
        `they draw as an empty box; use characters from ${scripts()}, or an \`icon=\`/\`image\``,
      ),
  );
}

/// The scripts the bundled fonts cover, as `A, B or C`.
function scripts(): string {
  const s = coveredScripts();
  if (s.length === 0) return "the bundled fonts";
  if (s.length === 1) return s[0]!;
  return `${s.slice(0, -1).join(", ")} or ${s[s.length - 1]!}`;
}
