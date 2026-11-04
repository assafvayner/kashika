import type { Span } from "../diag/index.js";
import { chars } from "../diag/index.js";

/// The source as the lexer counts it: lines of chars, without a leading byte order mark.
export class SourceText {
  private constructor(private readonly lineChars: string[][]) {}

  static new(src: string): SourceText {
    return SourceText.stripped(src.startsWith("\u{feff}") ? src.slice(1) : src);
  }

  /// The text of `src`, which has no byte order mark.
  static stripped(src: string): SourceText {
    return new SourceText(src.split("\n").map((l) => chars(l)));
  }

  lines(): string[][] {
    return this.lineChars;
  }

  /// The text of `span`.
  span(span: Span): string {
    return this.between(span, span);
  }

  /// The text from the start of `from` to the end of `to`, its lines joined by a space.
  /// Whatever lies outside the source is left out.
  between(from: Span, to: Span): string {
    let out = "";
    for (let line = from.line; line <= to.endLine; line++) {
      const cs = line >= 1 ? this.lineChars[line - 1] : undefined;
      if (cs === undefined) {
        break;
      }
      const a = line === from.line ? Math.max(from.col - 1, 0) : 0;
      const b = line === to.endLine ? Math.max(to.endCol - 1, 0) : cs.length;
      if (out !== "") {
        out += " ";
      }
      const end = Math.min(b, cs.length);
      if (a <= end) {
        out += cs.slice(a, end).join("");
      }
    }
    return out;
  }
}
