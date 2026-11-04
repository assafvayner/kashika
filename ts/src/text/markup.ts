import type { Json } from "../diag/json.js";
import { type Result, chars, err, ok } from "../diag/rust.js";

export type Font = "regular" | "bold" | "mono";

export class Run {
  constructor(
    public text: string,
    public font: Font,
  ) {}

  clone(): Run {
    return new Run(this.text, this.font);
  }

  toJson(): Json {
    return { font: this.font, text: this.text };
  }
}

export class MarkupError {
  constructor(public readonly marker: "**" | "`") {}

  toString(): string {
    return `unclosed \`${this.marker}\` in text`;
  }
}

type Mode = "plain" | "bold" | "code";

/// Width of a tab, in spaces. A tab has no glyph in the bundled fonts, so `parseMarkup`
/// replaces each one with this many spaces.
export const TAB_WIDTH = 4;

/// Splits `s` into lines of styled runs. `**bold**` and `` `code` `` don't nest: inside one,
/// the other marker is literal text. `\*` and `` \` `` are literal markers.
/// A tab becomes `TAB_WIDTH` spaces; there are no tab stops.
export function parseMarkup(s: string, bold: boolean): Result<Run[][], MarkupError> {
  const plain: Font = bold ? "bold" : "regular";
  const font = (m: Mode): Font => (m === "plain" ? plain : m === "bold" ? "bold" : "mono");
  const lines: Run[][] = [[]];
  let buf = "";
  let mode: Mode = "plain";
  const flush = (f: Font) => {
    if (buf === "") return;
    const line = lines[lines.length - 1]!;
    const last = line[line.length - 1];
    if (last !== undefined && last.font === f) last.text += buf;
    else line.push(new Run(buf, f));
    buf = "";
  };
  const cs = chars(s);
  let i = 0;
  while (i < cs.length) {
    const c = cs[i]!;
    const next = cs[i + 1];
    if (c === "\\" && (next === "*" || next === "`" || next === "\\")) {
      buf += next;
      i += 2;
      continue;
    } else if (c === "\t") {
      buf += " ".repeat(TAB_WIDTH);
    } else if (c === "\n") {
      flush(font(mode));
      lines.push([]);
    } else if (c === "*" && next === "*" && mode !== "code") {
      flush(font(mode));
      mode = mode === "bold" ? "plain" : "bold";
      i += 2;
      continue;
    } else if (c === "`" && mode !== "bold") {
      flush(font(mode));
      mode = mode === "code" ? "plain" : "code";
    } else {
      buf += c;
    }
    i += 1;
  }
  switch (mode) {
    case "plain":
      flush(font(mode));
      return ok(lines);
    case "bold":
      return err(new MarkupError("**"));
    case "code":
      return err(new MarkupError("`"));
  }
}
