import { Diagnostic, Span, chars, displayF64, isAsciiDigit, trimEnd } from "../diag/index.js";

export type Tok =
  | { kind: "Ident"; value: string }
  | { kind: "Num"; value: number }
  | { kind: "Str"; value: string }
  | { kind: "Color"; value: string }
  | {
      kind:
        | "LParen"
        | "RParen"
        | "LBrace"
        | "RBrace"
        | "Comma"
        | "Eq"
        | "Arrow"
        | "Dot"
        | "Plus"
        | "Minus"
        | "Colon"
        | "At"
        | "Newline"
        | "Eof";
    };

export type UnitTok = Exclude<Tok, { value: unknown }>["kind"];

export const Tok = {
  Ident: (value: string): Tok => ({ kind: "Ident", value }),
  Num: (value: number): Tok => ({ kind: "Num", value }),
  Str: (value: string): Tok => ({ kind: "Str", value }),
  Color: (value: string): Tok => ({ kind: "Color", value }),
  unit: (kind: UnitTok): Tok => ({ kind }),
};

export function tokEq(a: Tok, b: Tok): boolean {
  if (a.kind !== b.kind) return false;
  if ("value" in a && "value" in b) return a.value === b.value;
  return true;
}

export function describe(t: Tok): string {
  switch (t.kind) {
    case "Ident":
      return `\`${t.value}\``;
    case "Num":
      return `number \`${displayF64(t.value)}\``;
    case "Str":
      return "a string";
    case "Color":
      return `color \`${t.value}\``;
    case "LParen":
      return "`(`";
    case "RParen":
      return "`)`";
    case "LBrace":
      return "`{`";
    case "RBrace":
      return "`}`";
    case "Comma":
      return "`,`";
    case "Eq":
      return "`=`";
    case "Arrow":
      return "`->`";
    case "Dot":
      return "`.`";
    case "Plus":
      return "`+`";
    case "Minus":
      return "`-`";
    case "Colon":
      return "`:`";
    case "At":
      return "`@`";
    case "Newline":
      return "end of line";
    case "Eof":
      return "end of file";
  }
}

export interface Token {
  tok: Tok;
  span: Span;
}

export interface Comment {
  /// The comment's source text, including `//` or `/* */`; line comments are right-trimmed.
  text: string;
  span: Span;
}

export function lex(src: string): { toks: Token[]; diags: Diagnostic[] } {
  const { toks, diags } = lexWithComments(src);
  return { toks, diags };
}

/// Like `lex`, but also returns the comments, which the parser never sees.
export function lexWithComments(src: string): {
  toks: Token[];
  comments: Comment[];
  diags: Diagnostic[];
} {
  return lexRaw(src.startsWith("\u{feff}") ? src.slice(1) : src);
}

/// Lexes `src`, which has no byte order mark.
export function lexRaw(src: string): { toks: Token[]; comments: Comment[]; diags: Diagnostic[] } {
  const lx = new Lexer(chars(src));
  lx.run();
  return { toks: lx.toks, comments: lx.comments, diags: lx.diags };
}

function isAscii(c: string): boolean {
  return c.codePointAt(0)! < 0x80;
}

function isAsciiAlphanumeric(c: string): boolean {
  return /^[0-9A-Za-z]$/.test(c);
}

function isAsciiHexdigit(c: string): boolean {
  return /^[0-9A-Fa-f]$/.test(c);
}

function isAsciiAlpha(c: string): boolean {
  return /^[A-Za-z]$/.test(c);
}

const PUNCT: Record<string, UnitTok> = {
  "(": "LParen",
  ")": "RParen",
  "{": "LBrace",
  "}": "RBrace",
  ",": "Comma",
  "=": "Eq",
  ".": "Dot",
  "+": "Plus",
  ":": "Colon",
  "@": "At",
};

class Lexer {
  pos = 0;
  line = 1;
  col = 1;
  toks: Token[] = [];
  comments: Comment[] = [];
  diags: Diagnostic[] = [];

  constructor(private readonly chars: string[]) {}

  peek(n: number): string | null {
    return this.chars[this.pos + n] ?? null;
  }

  bump(): string | null {
    const c = this.chars[this.pos];
    if (c === undefined) return null;
    this.pos += 1;
    if (c === "\n") {
      this.line += 1;
      this.col = 1;
    } else {
      this.col += 1;
    }
    return c;
  }

  push(tok: Tok, line: number, col: number): void {
    this.toks.push({ tok, span: new Span(line, col, this.line, this.col) });
  }

  run(): void {
    for (let c = this.peek(0); c !== null; c = this.peek(0)) {
      const line = this.line;
      const col = this.col;
      if (c === " " || c === "\t" || c === "\r") {
        this.bump();
      } else if (c === "\n") {
        this.bump();
        this.toks.push({ tok: Tok.unit("Newline"), span: new Span(line, col, line, col + 1) });
      } else if (c === "/" && this.peek(1) === "/") {
        const start = this.pos;
        while (this.peek(0) !== null && this.peek(0) !== "\n") {
          this.bump();
        }
        const text = this.chars.slice(start, this.pos).join("");
        this.comments.push({
          text: trimEnd(text),
          span: new Span(line, col, this.line, this.col),
        });
      } else if (c === "/" && this.peek(1) === "*") {
        this.blockComment(line, col);
      } else if (c === '"') {
        this.string(line, col);
      } else if (c === "#" && this.isHashComment()) {
        this.diags.push(
          Diagnostic.error(
            "parse",
            new Span(line, col, line, col + 1),
            "`#` does not start a comment",
          ).withHelp("comments start with `//`; `#` only starts a color like `#rrggbb`"),
        );
        while (this.peek(0) !== null && this.peek(0) !== "\n") {
          this.bump();
        }
      } else if (c === "#") {
        this.color(line, col);
      } else if (c === "-" && this.peek(1) === ">") {
        this.bump();
        this.bump();
        this.push(Tok.unit("Arrow"), line, col);
      } else if (c === "-" && this.peek(1) !== null && isAsciiDigit(this.peek(1)!)) {
        this.number(line, col);
      } else if (c === "-") {
        this.bump();
        this.push(Tok.unit("Minus"), line, col);
      } else if (isAsciiDigit(c)) {
        this.number(line, col);
      } else if (isAsciiAlpha(c) || c === "_") {
        this.ident(line, col);
      } else if (c in PUNCT) {
        this.bump();
        this.push(Tok.unit(PUNCT[c]!), line, col);
      } else if (!isAscii(c)) {
        this.nonAscii(line, col);
      } else {
        this.bump();
        this.diags.push(
          Diagnostic.error(
            "parse",
            new Span(line, col, line, col + 1),
            `unexpected character \`${c}\``,
          ),
        );
      }
    }
    const line = this.line;
    const col = this.col;
    this.toks.push({ tok: Tok.unit("Eof"), span: new Span(line, col, line, col) });
  }

  /// Swallows a run of non-ASCII characters, and the ASCII identifier characters stuck
  /// to it, as one error.
  nonAscii(line: number, col: number): void {
    const last = this.toks[this.toks.length - 1];
    const inIdent =
      last !== undefined &&
      last.tok.kind === "Ident" &&
      last.span.endLine === line &&
      last.span.endCol === col;
    let run = "";
    for (;;) {
      const c = this.peek(0);
      if (c === null || !(!isAscii(c) || isAsciiAlphanumeric(c) || c === "_" || c === "-")) {
        break;
      }
      if (c === "-" && (this.peek(1) === ">" || this.peek(1) === "(")) {
        break;
      }
      run += c;
      this.bump();
    }
    const help = inIdent
      ? "ids are ASCII letters, digits, `_` and `-`"
      : "only ASCII is allowed outside strings and comments";
    this.diags.push(
      Diagnostic.error(
        "parse",
        new Span(line, col, this.line, this.col),
        `unexpected non-ASCII text \`${run}\``,
      ).withHelp(help),
    );
  }

  private digitAt(n: number): boolean {
    const c = this.peek(n);
    return c !== null && isAsciiDigit(c);
  }

  number(line: number, col: number): void {
    let s = "";
    if (this.peek(0) === "-") {
      s += "-";
      this.bump();
    }
    while (this.digitAt(0)) {
      s += this.peek(0)!;
      this.bump();
    }
    if (this.peek(0) === "." && this.digitAt(1)) {
      s += ".";
      this.bump();
      while (this.digitAt(0)) {
        s += this.peek(0)!;
        this.bump();
      }
    }
    const e = this.peek(0);
    const sign = this.peek(1);
    if (
      (e === "e" || e === "E") &&
      (this.digitAt(1) || ((sign === "+" || sign === "-") && this.digitAt(2)))
    ) {
      this.bump();
      this.bump();
      while (this.digitAt(0)) {
        this.bump();
      }
      this.diags.push(
        Diagnostic.error(
          "parse",
          new Span(line, col, this.line, this.col),
          "exponent notation is not supported",
        ).withHelp("write the number out in full, e.g. `100000` instead of `1e5`"),
      );
    }
    this.push(Tok.Num(Number(s)), line, col);
  }

  ident(line: number, col: number): void {
    let s = "";
    for (let c = this.peek(0); c !== null; c = this.peek(0)) {
      const continues =
        isAsciiAlphanumeric(c) ||
        c === "_" ||
        (c === "-" && !(this.peek(1) === ">" || this.peek(1) === "("));
      if (!continues) {
        break;
      }
      s += c;
      this.bump();
    }
    this.push(Tok.Ident(s), line, col);
  }

  string(line: number, col: number): void {
    this.bump();
    let s = "";
    for (;;) {
      const c = this.peek(0);
      if (c === null || c === "\n") {
        this.diags.push(
          Diagnostic.error(
            "parse",
            new Span(line, col, this.line, this.col),
            "unterminated string",
          ).withHelp('close the string with `"` before the end of the line'),
        );
        break;
      } else if (c === '"') {
        this.bump();
        break;
      } else if (c === "\\") {
        const el = this.line;
        const ec = this.col;
        this.bump();
        const next = this.peek(0);
        if (next === "n") {
          s += "\n";
        } else if (next === '"') {
          s += '"';
        } else if (next === "\\" || next === "*" || next === "`") {
          // `\\` and the markup escapes are kept for the text layer.
          s += "\\" + next;
        } else if (next !== null && next !== "\n") {
          s += next;
          this.diags.push(
            Diagnostic.error(
              "parse",
              new Span(el, ec, el, ec + 2),
              `unknown escape \`\\${next}\``,
            ).withHelp(
              'strings support `\\"`, `\\\\`, `\\n`, `\\*` and `\\``; write `\\\\` for a backslash',
            ),
          );
        } else {
          continue;
        }
        this.bump();
      } else {
        s += c;
        this.bump();
      }
    }
    this.push(Tok.Str(s), line, col);
  }

  blockComment(line: number, col: number): void {
    const start = this.pos;
    this.bump();
    this.bump();
    let multiline = false;
    for (;;) {
      const c = this.peek(0);
      if (c === null) {
        this.diags.push(
          Diagnostic.error(
            "parse",
            new Span(line, col, this.line, this.col),
            "unterminated block comment",
          ).withHelp("close it with `*/`"),
        );
        break;
      } else if (c === "*" && this.peek(1) === "/") {
        this.bump();
        this.bump();
        break;
      } else {
        multiline ||= c === "\n";
        this.bump();
      }
    }
    this.comments.push({
      text: this.chars.slice(start, this.pos).join(""),
      span: new Span(line, col, this.line, this.col),
    });
    if (multiline) {
      // A multi-line block comment ends its line; the Newline sits on the opening `/*`.
      this.toks.push({ tok: Tok.unit("Newline"), span: new Span(line, col, line, col + 2) });
    }
  }

  /// A `#` that begins a line, or stands alone outside an attribute value, is a
  /// mistaken comment rather than a color.
  isHashComment(): boolean {
    const last = this.toks[this.toks.length - 1];
    if (last === undefined || last.tok.kind === "Newline") return true;
    if (last.tok.kind === "Eq") return false;
    const next = this.peek(1);
    return !(next !== null && isAsciiAlphanumeric(next));
  }

  color(line: number, col: number): void {
    this.bump();
    let s = "#";
    for (let c = this.peek(0); c !== null && isAsciiAlphanumeric(c); c = this.peek(0)) {
      s += c;
      this.bump();
    }
    const hex = s.slice(1);
    if (!((hex.length === 3 || hex.length === 6 || hex.length === 8) && [...hex].every(isAsciiHexdigit))) {
      this.diags.push(
        Diagnostic.error(
          "parse",
          new Span(line, col, this.line, this.col),
          `invalid color \`${s}\``,
        ).withHelp("colors are `#rgb`, `#rrggbb` or `#rrggbbaa`; comments start with `//`"),
      );
    }
    this.push(Tok.Color(s), line, col);
  }
}
