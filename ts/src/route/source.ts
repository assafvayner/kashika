import { type Span } from "../diag/diagnostic.js";
import { SourceText } from "../syntax/index.js";

/// The source, and its text as the lexer counts it.
export class Source {
  private constructor(
    private readonly src: string,
    private readonly text: SourceText,
  ) {}

  static new(src: string): Source {
    return new Source(src, SourceText.new(src));
  }

  slice(span: Span): string {
    return this.text.span(span);
  }

  /// The source with `stmts` on lines of their own right after `after` (1-based line and
  /// exclusive column), or at the end when `null`, and the line the first one lands on.
  withLines(after: [number, number] | null, stmts: readonly string[]): [string, number] {
    const block = stmts.join("\n");
    if (after === null) {
      return [`${this.src}\n${block}\n`, this.text.lines().length + 1];
    }
    const [line, col] = after;
    let out = "";
    this.text.lines().forEach((l, i) => {
      if (i > 0) {
        out += "\n";
      }
      if (i + 1 === line) {
        const at = Math.min(col - 1, l.length);
        out += l.slice(0, at).join("");
        out += "\n";
        out += block;
        out += "\n";
        out += l.slice(at).join("");
      } else {
        out += l.join("");
      }
    });
    return [out, line + 1];
  }
}
