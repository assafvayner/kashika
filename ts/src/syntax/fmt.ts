import { Diagnostic, Span, chars, cmpTuple, err, ok } from "../diag/index.js";
import type { Result } from "../diag/index.js";
import { type Tok, type Token, lex, lexRaw, tokEq } from "./lexer.js";
import { parseStripped } from "./parser.js";

/// Rewrites `src` in canonical layout. Only whitespace changes: indentation, the spacing
/// between tokens, blank lines and trailing whitespace. Line breaks, comments and the
/// spelling of numbers, strings and colors are kept. Input with `parse` errors is refused.
export function format(input: string): Result<string, Diagnostic[]> {
  const src = input.startsWith("\u{feff}") ? input.slice(1) : input;
  const { diags } = parseStripped(src);
  const errors = diags.filter((d) => d.code === "parse");
  if (errors.length > 0) {
    return err(errors);
  }
  const { toks, comments } = lexRaw(src);
  const lines = src.split("\n").map((l) => chars(l));
  const items: Item[] = toks
    .filter((t) => !(t.tok.kind === "Newline" || t.tok.kind === "Eof"))
    .map((t) => new Item(tokText(t, lines), t.span, t.tok, false));
  items.push(
    ...comments.map(
      (c) => new Item(c.text.replaceAll("\r\n", "\n"), c.span, null, c.text.startsWith("//")),
    ),
  );
  items.sort((a, b) => cmpTuple([a.span.line, a.span.col], [b.span.line, b.span.col]));
  const out = print(items);
  if (!shapeEq(shape(lex(out).toks), shape(toks))) {
    return err([
      Diagnostic.error(
        "parse",
        Span.default(),
        "internal error: formatting would change the tokens; please report this file",
      ),
    ]);
  }
  return ok(out);
}

class Item {
  constructor(
    readonly text: string,
    readonly span: Span,
    readonly tok: Tok | null,
    readonly lineComment: boolean,
  ) {}

  is(kind: Tok["kind"]): boolean {
    return this.tok !== null && this.tok.kind === kind;
  }

  isIdent(s: string): boolean {
    return this.tok !== null && this.tok.kind === "Ident" && this.tok.value === s;
  }
}

function tokText(t: Token, lines: string[][]): string {
  switch (t.tok.kind) {
    case "Ident":
      return t.tok.value;
    case "Num":
    case "Str":
    case "Color": {
      const line = lines[t.span.line - 1]!;
      return line.slice(t.span.col - 1, t.span.endCol - 1).join("");
    }
    case "LParen":
      return "(";
    case "RParen":
      return ")";
    case "LBrace":
      return "{";
    case "RBrace":
      return "}";
    case "Comma":
      return ",";
    case "Eq":
      return "=";
    case "Arrow":
      return "->";
    case "Dot":
      return ".";
    case "Plus":
      return "+";
    case "Minus":
      return "-";
    case "Colon":
      return ":";
    case "At":
      return "@";
    case "Newline":
    case "Eof":
      return "";
  }
}

/// Token kinds with runs of newlines collapsed and leading and trailing newlines dropped.
function shape(toks: Token[]): Tok[] {
  const out: Tok[] = [];
  for (const t of toks) {
    const last = out[out.length - 1];
    if (t.tok.kind === "Newline" && (last === undefined || last.kind === "Newline")) {
      continue;
    }
    out.push(t.tok);
  }
  const n = out.length;
  if (n >= 2 && out[n - 2]!.kind === "Newline" && out[n - 1]!.kind === "Eof") {
    out.splice(n - 2, 1);
  }
  return out;
}

function shapeEq(a: Tok[], b: Tok[]): boolean {
  return a.length === b.length && a.every((t, i) => tokEq(t, b[i]!));
}

/// Groups items by source line; an item that starts on a line a previous item of the row
/// (such as a multi-line block comment) ends on joins that row.
function rows(items: Item[]): Item[][] {
  const out: Item[][] = [];
  let start = 0;
  let endLine = 0;
  items.forEach((it, i) => {
    if (i > start && it.span.line > endLine) {
      out.push(items.slice(start, i));
      start = i;
      endLine = 0;
    }
    endLine = Math.max(endLine, it.span.endLine);
  });
  if (start < items.length) {
    out.push(items.slice(start));
  }
  return out;
}

function print(items: Item[]): string {
  let out = "";
  // One entry per open `{`; `true` when it opened a `style` block, whose lines are attributes.
  const blocks: boolean[] = [];
  let prevEnd = 0;
  let prevOpened = false;
  for (const row of rows(items)) {
    const first = row[0]!;
    const closes = first.is("RBrace");
    const depth = Math.max(blocks.length - (closes ? 1 : 0), 0);
    const inStyle = blocks[blocks.length - 1] ?? false;
    if (prevEnd > 0 && first.span.line > prevEnd + 1 && !prevOpened && !closes) {
      out += "\n";
    }
    const cont = !inStyle && continues(row);
    out += "  ".repeat(depth + (cont ? 1 : 0));
    out += join(row);
    out += "\n";
    const style = first.isIdent("style");
    for (const it of row) {
      if (it.is("LBrace")) {
        blocks.push(style);
      } else if (it.is("RBrace")) {
        blocks.pop();
      }
    }
    const last = row[row.length - 1];
    prevOpened = last !== undefined && last.is("LBrace");
    prevEnd = row.length === 0 ? prevEnd : row.reduce((m, i) => Math.max(m, i.span.endLine), 0);
  }
  return out;
}

/// A physical line the parser joins onto the statement above it.
function continues(row: Item[]): boolean {
  const t = row[0]!.tok;
  if (t === null) return false;
  if (t.kind === "Arrow") return true;
  if (t.kind === "Ident") {
    const next = row[1]?.tok ?? null;
    return t.value === "ortho" || (next !== null && (next.kind === "Eq" || next.kind === "At"));
  }
  return false;
}

function join(row: Item[]): string {
  let s = "";
  let open = 0;
  row.forEach((it, i) => {
    if (i > 0) {
      if (it.lineComment) {
        s += "  ";
      } else if (space(row, i, open > 0)) {
        s += " ";
      }
    }
    if (it.is("LParen")) {
      open += 1;
    } else if (it.is("RParen")) {
      open -= 1;
    }
    s += it.text;
  });
  return s;
}

function space(row: Item[], i: number, inParens: boolean): boolean {
  const p = row[i - 1]!.tok;
  const n = row[i]!.tok;
  if (p === null || n === null) {
    return true;
  }
  switch (n.kind) {
    case "RParen":
    case "Comma":
    case "Dot":
    case "At":
    case "Eq":
    case "Colon":
    case "Plus":
    case "Minus":
      return false;
  }
  switch (p.kind) {
    case "Comma":
      return inParens;
    case "LParen":
    case "Dot":
    case "At":
    case "Eq":
    case "Plus":
    case "Minus":
    case "Colon":
      return false;
    default:
      return true;
  }
}
