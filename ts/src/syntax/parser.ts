import { Diagnostic, Span, didYouMean, displayF64, parseF64 } from "../diag/index.js";
import type { Result } from "../diag/index.js";
import type {
  Anchor,
  Attr,
  BlockText,
  Coord,
  Diagram,
  File,
  Ident,
  Junction,
  Line,
  Node,
  Offset,
  Point,
  Ref,
  Seg,
  SegKind,
  Size,
  Stmt,
  StyleDef,
  Target,
  Text,
  Value,
} from "./ast.js";
import { pointSpan } from "./ast.js";
import { type Tok, type Token, describe, lexRaw, tokEq } from "./lexer.js";
import { image, importStmt, shapeDef } from "./parser_defs.js";
import { SourceText } from "./source.js";

export const STATEMENTS: readonly string[] = [
  "diagram",
  "style",
  "box",
  "group",
  "junction",
  "text",
  "line",
  "arrow",
  "import",
  "shape",
  "image",
];
const LINE_FLAGS: readonly string[] = ["ortho"];
/// Segment keywords, which could not be bare segment targets, and the `arrow` statement keyword.
export const RESERVED_IDS: readonly string[] = [
  "h",
  "v",
  "x",
  "y",
  "hv",
  "vh",
  "vhv",
  "hvh",
  "curve",
  "arrow",
];
/// The reserved ids that are segment keywords.
export const SEGMENT_KEYWORDS: readonly string[] = [
  "h",
  "v",
  "x",
  "y",
  "hv",
  "vh",
  "vhv",
  "hvh",
  "curve",
];
const MAX_GROUP_DEPTH = 64;
const MAX_POINT_PARENS = 16;

export function parse(src: string): { file: File; diags: Diagnostic[] } {
  return parseStripped(src.startsWith("\u{feff}") ? src.slice(1) : src);
}

/// Parses `src`, which has no byte order mark.
export function parseStripped(src: string): { file: File; diags: Diagnostic[] } {
  const { toks, diags } = lexRaw(src);
  const first = firstStmt(toks);
  const p = new Parser(toks, src);
  const stmts = p.stmts(false);
  diags.push(...p.diags);
  const file: File = {
    stmts,
    first_stmt: first,
    failed_ids: p.failedIds,
    failed_diagram: p.failedDiagram,
  };
  return { file, diags };
}

/// Where the first statement starts, skipping a misplaced `kashika=` line, which is
/// reported on its own.
function firstStmt(toks: Token[]): Span | null {
  let i = 0;
  for (;;) {
    const t = toks[i];
    if (t === undefined) return null;
    const tok = t.tok;
    if (tok.kind === "Newline") {
      i += 1;
    } else if (tok.kind === "Eof") {
      return null;
    } else if (tok.kind === "Ident" && tok.value === "kashika") {
      const next = toks[i + 1];
      if (next === undefined) return null;
      if (next.tok.kind !== "Eq") return t.span;
      for (;;) {
        const u = toks[i];
        if (u === undefined) return null;
        if (u.tok.kind === "Newline" || u.tok.kind === "Eof") break;
        i += 1;
      }
    } else {
      return t.span;
    }
  }
}

export function startsPoint(t: Tok): boolean {
  return t.kind === "Ident" || t.kind === "LParen";
}

/// The `Err(())` of Rust's `PResult`: thrown by a failing parse step and caught where the
/// Rust matches on the result.
export const FAIL: unique symbol = Symbol("parse failed");

export function isFail(e: unknown): e is typeof FAIL {
  return e === FAIL;
}

export interface Clauses {
  at: boolean;
  size: boolean;
}

export const Clauses = {
  AT: { at: true, size: false } as Clauses,
  SIZE: { at: false, size: true } as Clauses,
  AT_SIZE: { at: true, size: true } as Clauses,
};

interface Tail {
  at: Point | null;
  /// The size and the span of its clause.
  size: [Size, Span] | null;
  attrs: Attr[];
}

interface Slot<T> {
  v: T | null;
}

export class Parser {
  pos = 0;
  depth = 0;
  /// The parentheses enclosing the point being parsed.
  pointParens = 0;
  diags: Diagnostic[] = [];
  private sourceText: SourceText | null = null;
  skipRecover = false;
  /// Ids read by the statements being parsed, kept for an enclosing group until it parses.
  declared: Ident[] = [];
  failedIds: Ident[] = [];
  failedDiagram = false;

  constructor(
    readonly toks: Token[],
    readonly src: string,
  ) {}

  tok(): Tok {
    return this.toks[this.pos]!.tok;
  }

  tokAt(n: number): Tok {
    return this.toks[Math.min(this.pos + n, this.toks.length - 1)]!.tok;
  }

  span(): Span {
    return this.toks[this.pos]!.span;
  }

  prevSpan(): Span {
    return this.toks[Math.max(this.pos - 1, 0)]!.span;
  }

  bump(): Span {
    const span = this.toks[this.pos]!.span;
    if (this.pos < this.toks.length - 1) {
      this.pos += 1;
    }
    return span;
  }

  /// Runs `f`, turning a parse failure into an `Err`.
  attempt<T>(f: () => T): Result<T, null> {
    try {
      return { ok: true, value: f() };
    } catch (e) {
      if (isFail(e)) return { ok: false, error: null };
      throw e;
    }
  }

  /// Records `expected {expected}, found ...` without failing.
  pushExpected(expected: string): void {
    const found = describe(this.tok());
    this.diags.push(Diagnostic.error("parse", this.span(), `expected ${expected}, found ${found}`));
  }

  error(expected: string): never {
    this.pushExpected(expected);
    throw FAIL;
  }

  /// `error`, with a help line.
  errorHelp(expected: string, help: string): never {
    this.pushExpected(expected);
    const d = this.diags.pop();
    if (d !== undefined) {
      this.diags.push(d.withHelp(help));
    }
    throw FAIL;
  }

  /// The source text from the start of `from` to the end of `to`.
  source(from: Span, to: Span): string {
    if (this.sourceText === null) {
      this.sourceText = SourceText.stripped(this.src);
    }
    return this.sourceText.between(from, to);
  }

  /// The span of the last token before the end of the current physical line.
  lineEnd(): Span {
    let i = this.pos;
    while (i + 1 < this.toks.length) {
      const k = this.toks[i + 1]!.tok.kind;
      if (k === "Newline" || k === "Eof") break;
      i += 1;
    }
    return this.toks[i]!.span;
  }

  isKw(kw: string): boolean {
    const t = this.tok();
    return t.kind === "Ident" && t.value === kw;
  }

  skipNewlines(): void {
    while (this.tok().kind === "Newline") {
      this.bump();
    }
  }

  stmts(inBlock: boolean): Stmt[] {
    const out: Stmt[] = [];
    for (;;) {
      this.skipNewlines();
      const k = this.tok().kind;
      if (k === "Eof") {
        if (inBlock) {
          this.pushExpected("`}`");
        }
        break;
      }
      if (k === "RBrace" && inBlock) break;
      if (k === "RBrace") {
        const last = this.diags[this.diags.length - 1];
        if (last === undefined || !last.span.eq(this.span())) {
          this.diags.push(Diagnostic.error("parse", this.span(), "unexpected `}`"));
        }
        this.bump();
        continue;
      }
      const r = this.attempt(() => this.stmt());
      if (r.ok) {
        if (r.value !== null) out.push(r.value);
      } else {
        const skip = this.skipRecover;
        this.skipRecover = false;
        if (!skip) {
          this.recover();
        }
      }
    }
    return out;
  }

  /// Skips the rest of a broken statement: up to the next newline at brace depth 0,
  /// or up to (not including) a `}` that closes the enclosing block.
  recover(): void {
    let depth = 0;
    for (;;) {
      const k = this.tok().kind;
      if (k === "Eof") return;
      if (k === "Newline" && depth === 0) {
        this.bump();
        return;
      }
      if (k === "RBrace" && depth === 0) return;
      if (k === "LBrace") depth += 1;
      else if (k === "RBrace") depth -= 1;
      this.bump();
    }
  }

  /// Parses a statement; when it fails, records whether it was the `diagram` statement and
  /// the ids it declared, including those of a dropped group's children.
  stmt(): Stmt | null {
    const mark = this.declared.length;
    const diagram = this.isKw("diagram");
    let result: Stmt | null;
    try {
      result = this.stmtBody();
    } catch (e) {
      if (isFail(e)) {
        this.failedDiagram ||= diagram;
        this.failedIds.push(...this.declared.splice(mark));
      }
      throw e;
    }
    if (this.depth === 0) {
      this.declared.length = mark;
    }
    return result;
  }

  stmtBody(): Stmt | null {
    const start = this.span();
    const t = this.tok();
    if (t.kind !== "Ident") {
      return this.error("a statement");
    }
    const kw = t.value;
    if (kw === "kashika" && this.tokAt(1).kind === "Eq") {
      const end = this.lineEnd();
      const version =
        this.tokAt(2).kind === "Str"
          ? this.source(this.toks[this.pos + 2]!.span, end)
          : '"X.Y.Z"';
      this.diags.push(
        Diagnostic.error("parse", start.to(end), "`kashika=` is not a statement").withHelp(
          `the version is an attribute of \`diagram\`: write \`diagram "…" size (w,h) kashika=${version}\``,
        ),
      );
      throw FAIL;
    }
    let stmt: Stmt;
    switch (kw) {
      case "diagram":
        this.bump();
        stmt = { kind: "Diagram", value: this.diagram(start) };
        break;
      case "style":
        this.bump();
        stmt = { kind: "Style", value: this.styleDef(start) };
        break;
      case "box": {
        this.bump();
        const node = this.node(start);
        if (this.tok().kind === "LBrace") {
          node.block = this.textBlock();
        }
        stmt = { kind: "Box", value: node };
        break;
      }
      case "group": {
        if (this.depth >= MAX_GROUP_DEPTH) {
          const closed = this.skipUnbuilt();
          this.diags.push(
            Diagnostic.error(
              "parse",
              start.to(this.prevSpan()),
              `groups are nested more than ${MAX_GROUP_DEPTH} deep`,
            ).withHelp("flatten the hierarchy; ids are global, so children need no parent"),
          );
          if (!closed) {
            return this.error("`}`");
          }
          this.endOfStmt();
          return null;
        }
        this.bump();
        const node = this.node(start);
        this.depth += 1;
        let children: Stmt[];
        try {
          children = this.block();
        } finally {
          this.depth -= 1;
        }
        stmt = { kind: "Group", value: { node, children } };
        break;
      }
      case "junction":
        this.bump();
        stmt = { kind: "Junction", value: this.junction(start) };
        break;
      case "text":
        this.bump();
        stmt = { kind: "Text", value: this.text(start) };
        break;
      case "line":
      case "arrow":
        this.bump();
        stmt = { kind: "Line", value: this.line(start, kw === "arrow") };
        break;
      case "import":
        this.bump();
        stmt = { kind: "Import", value: importStmt(this, start) };
        break;
      case "shape":
        this.bump();
        stmt = { kind: "Shape", value: shapeDef(this, start) };
        break;
      case "image":
        this.bump();
        stmt = { kind: "Image", value: image(this, start) };
        break;
      default: {
        let d = Diagnostic.error(
          "parse",
          this.span(),
          `expected a statement (${STATEMENTS.join(", ")}), found \`${kw}\``,
        );
        const s = didYouMean(kw, STATEMENTS);
        if (s !== null) {
          d = d.withHelp(`did you mean \`${s}\`?`);
        }
        this.diags.push(d);
        throw FAIL;
      }
    }
    this.endOfStmt();
    return stmt;
  }

  /// Skips a statement the parser won't build: up to the end of the line, or past the
  /// `}` that matches any `{` it opens, across lines.
  /// Returns `false` if the input ends inside an open `{`.
  skipUnbuilt(): boolean {
    let depth = 0;
    for (;;) {
      const k = this.tok().kind;
      if (k === "Eof") return depth === 0;
      if ((k === "Newline" || k === "RBrace") && depth === 0) return true;
      if (k === "LBrace") depth += 1;
      else if (k === "RBrace") depth -= 1;
      this.bump();
    }
  }

  endOfStmt(): void {
    const k = this.tok().kind;
    if (k === "Newline") {
      this.bump();
      return;
    }
    if (k === "Eof" || k === "RBrace") return;
    this.error("end of statement");
  }

  block(): Stmt[] {
    if (this.tok().kind !== "LBrace") {
      return this.error("`{`");
    }
    this.bump();
    const children = this.stmts(true);
    if (this.tok().kind === "RBrace") {
      this.bump();
    }
    return children;
  }

  ident(what: string): Ident {
    const t = this.tok();
    if (t.kind === "Ident") {
      const name = t.value;
      const span = this.bump();
      return { name, span };
    }
    return this.error(what);
  }

  /// An id being declared; segment keywords are reserved, and a statement declaring one
  /// is dropped.
  declId(what: string): Ident {
    const id = this.ident(what);
    if (RESERVED_IDS.includes(id.name)) {
      this.diags.push(
        Diagnostic.error(
          "parse",
          id.span,
          `\`${id.name}\` is a reserved word and cannot be an id`,
        ).withHelp(
          SEGMENT_KEYWORDS.includes(id.name)
            ? `segment keywords (${SEGMENT_KEYWORDS.join(" ")}) are reserved; pick another name, e.g. \`${id.name}_1\``
            : `\`${id.name}\` is a statement keyword; pick another name, e.g. \`${id.name}_1\``,
        ),
      );
      throw FAIL;
    }
    this.declared.push({ name: id.name, span: id.span });
    return id;
  }

  expect(tok: Tok): Span {
    if (tokEq(this.tok(), tok)) {
      return this.bump();
    }
    return this.error(describe(tok));
  }

  number(): number {
    const t = this.tok();
    if (t.kind === "Num") {
      const n = t.value;
      this.bump();
      return n;
    }
    return this.error("a number");
  }

  string(): string {
    const t = this.tok();
    if (t.kind === "Str") {
      this.bump();
      return t.value;
    }
    return this.error("a string");
  }

  pair(): [number, number] {
    this.expect({ kind: "LParen" });
    const a = this.number();
    this.expect({ kind: "Comma" });
    const b = this.number();
    this.expect({ kind: "RParen" });
    return [a, b];
  }

  /// `size (w,h)`, where each axis is a number or `auto`.
  size(): Size {
    const help = this.bareSize();
    if (help !== null) {
      return this.errorHelp("`(`", help);
    }
    this.expect({ kind: "LParen" });
    const w = this.dim();
    this.expect({ kind: "Comma" });
    const h = this.dim();
    this.expect({ kind: "RParen" });
    return { w, h };
  }

  /// The help for a size written without parentheses: `100x50`, `100 x 50` or `100,50`.
  bareSize(): string | null {
    const t = this.tok();
    if (t.kind !== "Num") {
      return null;
    }
    const w = t.value;
    const t1 = this.tokAt(1);
    const t2 = this.tokAt(2);
    let h: number;
    if (t1.kind === "Ident" && t1.value === "x" && t2.kind === "Num") {
      h = t2.value;
    } else if (t1.kind === "Comma" && t2.kind === "Num") {
      h = t2.value;
    } else if (t1.kind === "Ident") {
      if (!t1.value.startsWith("x")) return null;
      const v = parseF64(t1.value.slice(1));
      if (v === null) return null;
      h = v;
    } else {
      return null;
    }
    return `write \`size (${displayF64(w)},${displayF64(h)})\``;
  }

  dim(): number | null {
    if (this.isKw("auto")) {
      this.bump();
      return null;
    }
    return this.number();
  }

  /// A required size with no `auto` axis, for an image.
  fixedSize(size: [Size, Span] | null, what: string): [number, number] {
    const [s, span] = this.required(size, "size");
    if (s.w !== null && s.h !== null) {
      return [s.w, s.h];
    }
    this.diags.push(
      Diagnostic.error("parse", span, `${what} size can't be \`auto\``).withHelp(
        "only boxes, groups and the diagram take `auto`; give it a size: `size (w,h)`",
      ),
    );
    throw FAIL;
  }

  diagram(start: Span): Diagram {
    const title = this.string();
    const tail = this.tail(Clauses.SIZE);
    return {
      title,
      size: tail.size === null ? null : tail.size[0],
      attrs: tail.attrs,
      span: start.to(this.prevSpan()),
    };
  }

  styleDef(start: Span): StyleDef {
    const name = this.ident("a style name");
    this.expect({ kind: "LBrace" });
    const attrs: Attr[] = [];
    for (;;) {
      this.skipNewlines();
      const k = this.tok().kind;
      if (k === "RBrace") {
        this.bump();
        break;
      }
      if (k === "Eof") {
        return this.error("`}`");
      }
      const r = this.attempt(() => this.attr());
      if (r.ok) {
        attrs.push(r.value);
      } else {
        this.recover();
      }
    }
    return { name: name.name, attrs, span: start.to(this.prevSpan()) };
  }

  node(start: Span): Node {
    const id = this.declId("an id");
    const t = this.tok();
    let label: string | null = null;
    if (t.kind === "Str") {
      this.bump();
      label = t.value;
    }
    const tail = this.tail(Clauses.AT_SIZE);
    const at = this.required(tail.at, "at");
    const size = tail.size === null ? null : tail.size[0];
    return {
      id,
      label,
      at,
      size,
      attrs: tail.attrs,
      block: null,
      span: start.to(this.prevSpan()),
    };
  }

  /// Parses a box's `{ }` text block: `text "<content>" [at <point>] [attrs]` per line.
  textBlock(): BlockText[] {
    this.bump();
    const items: BlockText[] = [];
    for (;;) {
      this.skipNewlines();
      const k = this.tok().kind;
      if (k === "RBrace") {
        this.bump();
        return items;
      }
      if (k === "Eof") {
        return this.error("`}`");
      }
      const r = this.attempt(() => this.blockText());
      if (r.ok) {
        items.push(r.value);
      } else {
        this.recover();
      }
    }
  }

  blockText(): BlockText {
    const start = this.span();
    if (!this.isKw("text")) {
      return this.error("`text` or `}` in a text block");
    }
    this.bump();
    const t = this.tok();
    if (t.kind === "Ident") {
      this.diags.push(
        Diagnostic.error("parse", this.span(), "text block items take no id").withHelp(
          `remove \`${t.value}\``,
        ),
      );
      throw FAIL;
    }
    const content = this.string();
    const tail = this.tail(Clauses.AT);
    const span = start.to(this.prevSpan());
    const k = this.tok().kind;
    if (k === "Newline" || k === "RBrace") {
      return { content, at: tail.at, attrs: tail.attrs, span };
    }
    return this.error("end of statement");
  }

  junction(start: Span): Junction {
    const id = this.declId("an id");
    const tail = this.tail(Clauses.AT);
    return {
      id,
      at: this.required(tail.at, "at"),
      attrs: tail.attrs,
      span: start.to(this.prevSpan()),
    };
  }

  text(start: Span): Text {
    const id = this.tok().kind === "Ident" ? this.declId("an id") : null;
    const content = this.string();
    const tail = this.tail(Clauses.AT);
    return {
      id,
      content,
      at: this.required(tail.at, "at"),
      attrs: tail.attrs,
      span: start.to(this.prevSpan()),
    };
  }

  /// A line id is an identifier followed by something that starts a point; a point that
  /// starts with an identifier never continues with another identifier or `(`.
  line(start: Span, arrow: boolean): Line {
    const kw = arrow ? "arrow" : "line";
    let colonDiag: number | null = null;
    let id: Ident | null;
    if (this.tok().kind === "Ident" && this.tokAt(1).kind === "Colon") {
      const i = this.declId("a line id");
      const colon = this.bump();
      const k = this.tok().kind;
      const rest =
        k === "Newline" || k === "Eof"
          ? "a.right -> b.left"
          : this.source(this.span(), this.lineEnd());
      colonDiag = this.diags.length;
      this.diags.push(
        Diagnostic.error("parse", colon, "a line id takes no `:`").withHelp(
          `remove the \`:\`: write \`${kw} ${i.name} ${rest}\``,
        ),
      );
      id = i;
    } else if (this.tok().kind === "Ident" && startsPoint(this.tokAt(1))) {
      id = this.declId("a line id");
    } else {
      id = null;
    }
    const fromKw = this.isKw("from") && startsPoint(this.tokAt(1));
    if (fromKw) {
      this.bump();
    }
    const startPt = this.point();
    const toKw = this.isKw("to") && startsPoint(this.tokAt(1));
    if (fromKw || toKw) {
      if (!fromKw && id !== null && id.name === "from") {
        id = null;
      }
      const a = this.source(pointSpan(startPt), pointSpan(startPt));
      let b: string;
      if (toKw) {
        this.bump();
        const end = this.point();
        b = this.source(pointSpan(end), pointSpan(end));
      } else {
        b = "b.left";
      }
      if (colonDiag !== null) {
        this.diags.splice(colonDiag, 1);
      }
      const idText = id === null ? "" : `${id.name} `;
      this.diags.push(
        Diagnostic.error(
          "parse",
          start.to(this.prevSpan()),
          "a line is written with `->`, not `from … to`",
        ).withHelp(`write \`${kw} ${idText}${a} -> ${b}\``),
      );
      throw FAIL;
    }
    const segs: Seg[] = [];
    for (;;) {
      this.continuation(true, LINE_FLAGS);
      if (this.tok().kind !== "Arrow") {
        break;
      }
      const arrowSpan = this.bump();
      const k = this.tok().kind;
      if (k === "Newline" || k === "Eof") {
        return this.danglingArrow(arrowSpan);
      }
      segs.push(this.seg());
    }
    if (segs.length === 0) {
      if (this.tok().kind === "Eq") {
        return this.errorHelp("`->` followed by a segment", "write `->`");
      }
      return this.error("`->` followed by a segment");
    }
    const attrs = this.attrs(LINE_FLAGS);
    return {
      id,
      arrow,
      start: startPt,
      segs,
      attrs,
      span: start.to(this.prevSpan()),
    };
  }

  /// Reports a `->` that ends its line. A next line that is only a segment target is
  /// swallowed with it, so one mistake is one error.
  danglingArrow(arrow: Span): never {
    this.diags.push(
      Diagnostic.error(
        "parse",
        arrow,
        "`->` must be followed by a segment on the same line",
      ).withHelp(
        "to continue a line, start the next physical line with `->`, " +
          "e.g. `line a x.right` then `  -> y.left`",
      ),
    );
    let n = 0;
    while (this.tokAt(n).kind === "Newline") {
      n += 1;
    }
    const t = this.tokAt(n);
    let target: boolean;
    if (t.kind === "LParen") {
      target = true;
    } else if (t.kind === "Ident") {
      const k = this.tokAt(n + 1).kind;
      target =
        !STATEMENTS.includes(t.value) &&
        (k === "Dot" || k === "Plus" || k === "Minus" || k === "Newline" || k === "Eof");
    } else {
      target = false;
    }
    if (target) {
      for (let i = 0; i < n; i++) {
        this.bump();
      }
      this.recover();
      this.skipRecover = true;
    }
    throw FAIL;
  }

  /// Joins the next physical line onto this statement when it starts with `->`
  /// (if `allowArrow`), with an attribute (`key=` or `key@`), or with one of `flags`.
  continuation(allowArrow: boolean, flags: readonly string[]): void {
    if (this.tok().kind !== "Newline") {
      return;
    }
    let n = 0;
    while (this.tokAt(n).kind === "Newline") {
      n += 1;
    }
    const t = this.tokAt(n);
    const next = this.tokAt(n + 1).kind;
    const continues =
      (allowArrow && t.kind === "Arrow") ||
      (t.kind === "Ident" && (next === "Eq" || next === "At")) ||
      (t.kind === "Ident" && flags.includes(t.value));
    if (continues) {
      for (let i = 0; i < n; i++) {
        this.bump();
      }
    }
  }

  /// Parses `at <point>` and `size (w,h)`, where `clauses` allows them, and attributes,
  /// in any order. A repeated `at` or `size` is an error.
  tail(clauses: Clauses): Tail {
    const at: Slot<Point> = { v: null };
    const size: Slot<[Size, Span]> = { v: null };
    const attrs: Attr[] = [];
    for (;;) {
      this.continuation(false, []);
      const k1 = this.tokAt(1).kind;
      const clause = !(k1 === "Eq" || k1 === "At");
      if (clauses.at && clause && this.isKw("at")) {
        const kw = this.bump();
        const p = this.point();
        this.once(at, p, "at", kw);
      } else if (clauses.size && clause && this.isKw("size")) {
        const kw = this.bump();
        const s = this.size();
        const span = kw.to(this.prevSpan());
        this.once(size, [s, span], "size", kw);
      } else if (this.tok().kind === "Ident") {
        attrs.push(this.attr());
      } else {
        return { at: at.v, size: size.v, attrs };
      }
    }
  }

  once<T>(slot: Slot<T>, v: T, kw: string, span: Span): void {
    if (slot.v !== null) {
      this.diags.push(
        Diagnostic.error("parse", span, `duplicate \`${kw}\``).withHelp(
          `a statement takes one \`${kw}\`; remove the other`,
        ),
      );
      throw FAIL;
    }
    slot.v = v;
  }

  required<T>(v: T | null, kw: string): T {
    if (v !== null) return v;
    if (kw === "at" && this.tok().kind === "LParen") {
      return this.errorHelp("`at`", "a position starts with `at`: `at (x,y)`");
    }
    return this.error(`\`${kw}\``);
  }

  attrs(flags: readonly string[]): Attr[] {
    const out: Attr[] = [];
    for (;;) {
      this.continuation(false, flags);
      if (this.tok().kind !== "Ident") {
        return out;
      }
      out.push(this.attr());
    }
  }

  attr(): Attr {
    const key = this.ident("an attribute");
    let atT: number | null = null;
    if (this.tok().kind === "At") {
      this.bump();
      atT = this.number();
    }
    let value: Value;
    if (this.tok().kind === "Eq") {
      this.bump();
      let v: Value;
      if (key.name === "size" && this.tok().kind === "LParen") {
        v = { kind: "Size", value: this.size() };
      } else {
        v = this.value();
      }
      if (key.name === "pin" && this.tok().kind === "At") {
        this.bump();
        const t = this.number();
        if (v.kind === "Word") {
          value = { kind: "AnchorAt", value: [v.value, t] };
        } else {
          return this.error("an anchor name before `@`");
        }
      } else {
        value = v;
      }
    } else if (atT !== null) {
      return this.error("`=`");
    } else {
      value = { kind: "Flag" };
    }
    const span = key.span.to(this.prevSpan());
    return { key, at_t: atT, value, span };
  }

  value(): Value {
    const t = this.tok();
    switch (t.kind) {
      case "Num":
        this.bump();
        return { kind: "Num", value: t.value };
      case "Str":
        this.bump();
        return { kind: "Str", value: t.value };
      case "Color":
        this.bump();
        return { kind: "Color", value: t.value };
      case "LParen": {
        const [a, b] = this.pair();
        return { kind: "Pair", value: [a, b] };
      }
      case "Ident": {
        this.bump();
        let word = t.value;
        for (;;) {
          const k = this.tok().kind;
          let sep: string;
          if (k === "Colon") {
            sep = ":";
          } else if (k === "Comma") {
            sep = ",";
          } else if (k === "Dot" && this.tokAt(1).kind === "Ident") {
            sep = ".";
          } else {
            break;
          }
          this.bump();
          const n = this.tok();
          if (n.kind === "Ident") {
            this.bump();
            word += sep + n.value;
          } else {
            return this.error("an identifier");
          }
        }
        return { kind: "Word", value: word };
      }
      default:
        return this.error("an attribute value");
    }
  }

  /// A point: a literal, mixed, reference or grouped point, then any number of offsets.
  point(): Point {
    const start = this.span();
    let base: Point;
    if (this.tok().kind === "LParen") {
      if (this.pointParens === MAX_POINT_PARENS) {
        return this.tooManyParens();
      }
      this.pointParens += 1;
      try {
        base = this.parenIsGroup() ? this.group() : this.literal();
      } finally {
        this.pointParens -= 1;
      }
    } else {
      const r = this.reference();
      const offset = this.offset();
      base = { kind: "Ref", r, offset, span: start.to(this.prevSpan()) };
    }
    const offsets: Offset[] = [];
    for (let o = this.offset(); o !== null; o = this.offset()) {
      offsets.push(o);
    }
    if (offsets.length === 0) {
      return base;
    }
    return { kind: "Shifted", base, offsets, span: start.to(this.prevSpan()) };
  }

  offset(): Offset | null {
    const k = this.tok().kind;
    if (!(k === "Plus" || k === "Minus")) {
      return null;
    }
    const neg = k === "Minus";
    this.bump();
    if (neg && this.tok().kind === "Arrow") {
      return this.errorHelp("`(`", "a line hop is `->`, not `-->`");
    }
    const [dx, dy] = this.pair();
    return { neg, dx, dy };
  }

  /// `(x,y)`, where each coordinate is a number or one axis of a reference.
  literal(): Point {
    const start = this.bump();
    const x = this.coord();
    this.expect({ kind: "Comma" });
    const y = this.coord();
    this.expect({ kind: "RParen" });
    return { kind: "Lit", x, y, span: start.to(this.prevSpan()) };
  }

  /// Whether the `(` here opens a grouped point rather than `(x,y)`: it is followed by `(`
  /// or a reference, and no `,` sits directly inside it before its closing `)`. The scan
  /// stops at the end of the line, and the nesting limit bounds how often a token is rescanned.
  parenIsGroup(): boolean {
    const k1 = this.tokAt(1).kind;
    if (!(k1 === "LParen" || k1 === "Ident")) {
      return false;
    }
    let depth = 0;
    let n = 1;
    for (;;) {
      const k = this.tokAt(n).kind;
      if (k === "LParen") {
        depth += 1;
      } else if (k === "RParen" && depth === 0) {
        return true;
      } else if (k === "RParen") {
        depth -= 1;
      } else if (k === "Comma" && depth === 0) {
        return false;
      } else if (k === "Newline" || k === "Eof") {
        return true;
      }
      n += 1;
    }
  }

  group(): Point {
    const open = this.bump();
    const inner = this.point();
    if (this.tok().kind !== "RParen") {
      const found = describe(this.tok());
      const written = this.source(open, pointSpan(inner));
      const t1 = this.tokAt(1);
      const help =
        this.tok().kind === "Dot" && t1.kind === "Ident" && (t1.value === "x" || t1.value === "y")
          ? `\`.x\`/\`.y\` take one axis of a mixed point, which needs both: \`${written}.x, other.y)\``
          : `close it: \`${written})\``;
      this.diags.push(
        Diagnostic.error(
          "parse",
          this.span(),
          `expected \`)\` to close the \`(\` around the point, found ${found}`,
        ).withHelp(help),
      );
      throw FAIL;
    }
    this.bump();
    return { kind: "Group", inner, span: open.to(this.prevSpan()) };
  }

  /// Reports a `(` nested deeper than the limit, spanning it to its `)` on the same line.
  tooManyParens(): never {
    const start = this.span();
    let depth = 0;
    let end = start;
    for (let i = this.pos; i < this.toks.length; i++) {
      const t = this.toks[i]!;
      const k = t.tok.kind;
      if (k === "LParen") {
        depth += 1;
      } else if (k === "RParen") {
        depth -= 1;
        if (depth === 0) {
          end = t.span;
          break;
        }
      } else if (k === "Newline" || k === "Eof") {
        break;
      }
    }
    this.diags.push(
      Diagnostic.error(
        "parse",
        start.to(end),
        `a point nests at most ${MAX_POINT_PARENS} sets of parentheses`,
      ).withHelp("parentheses around a point change nothing; remove the extra ones"),
    );
    throw FAIL;
  }

  coord(): Coord {
    const t = this.tok();
    if (t.kind === "Num") {
      const n = t.value;
      this.bump();
      return { kind: "Num", value: n };
    }
    if (t.kind !== "Ident") {
      return this.error("a number or a reference like `db.left.x`");
    }
    const r = this.reference();
    this.expect({ kind: "Dot" });
    const a = this.tok();
    let axis: "X" | "Y";
    if (a.kind === "Ident" && a.value === "x") {
      axis = "X";
    } else if (a.kind === "Ident" && a.value === "y") {
      axis = "Y";
    } else {
      return this.error("`x` or `y`");
    }
    this.bump();
    return { kind: "Axis", r, axis };
  }

  reference(): Ref {
    const EXPECTED = "a point: `(x,y)` or a reference like `db.left`";
    const t0 = this.tok();
    const t1 = this.tokAt(1);
    const t2 = this.tokAt(2);
    if (t0.kind === "Num" && t1.kind === "Comma" && t2.kind === "Num") {
      const help = `a literal point is in parentheses: \`(${displayF64(t0.value)},${displayF64(t2.value)})\``;
      return this.errorHelp(EXPECTED, help);
    }
    const id = this.ident(EXPECTED);
    const n = this.tokAt(1);
    const nextIsAnchor =
      this.tok().kind === "Dot" && n.kind === "Ident" && n.value !== "x" && n.value !== "y";
    let anchor: Anchor | null = null;
    if (nextIsAnchor) {
      this.bump();
      const name = this.ident("an anchor");
      let t: number | null = null;
      if (this.tok().kind === "At") {
        this.bump();
        t = this.number();
      }
      anchor = { name, t };
    }
    return { id, anchor };
  }

  seg(): Seg {
    const start = this.span();
    if (this.isKw("to") && startsPoint(this.tokAt(1))) {
      const to = this.bump();
      const p = this.point();
      const text = this.source(pointSpan(p), pointSpan(p));
      this.diags.push(
        Diagnostic.error("parse", to, "`->` takes the point directly, with no `to`").withHelp(
          `write \`-> ${text}\``,
        ),
      );
      throw FAIL;
    }
    const t = this.tok();
    const kw = t.kind === "Ident" && this.tokAt(1).kind !== "Dot" ? t.value : null;
    let kind: SegKind;
    switch (kw) {
      case "h":
        this.bump();
        kind = { kind: "H", value: this.number() };
        break;
      case "v":
        this.bump();
        kind = { kind: "V", value: this.number() };
        break;
      case "x":
        this.bump();
        kind = { kind: "X", value: this.target("x") };
        break;
      case "y":
        this.bump();
        kind = { kind: "Y", value: this.target("y") };
        break;
      case "hv":
        this.bump();
        kind = { kind: "Hv", value: this.point() };
        break;
      case "vh":
        this.bump();
        kind = { kind: "Vh", value: this.point() };
        break;
      case "vhv":
      case "hvh": {
        const vertical = kw === "vhv";
        this.bump();
        const tt = this.crossAt(vertical);
        const to = this.point();
        kind = vertical ? { kind: "Vhv", t: tt, to } : { kind: "Hvh", t: tt, to };
        break;
      }
      case "curve":
        this.bump();
        kind = this.curve();
        break;
      default:
        kind = { kind: "To", value: this.point() };
    }
    return { kind, span: start.to(this.prevSpan()) };
  }

  /// The `@t` of `vhv@t`/`hvh@t`: where the middle run sits, as a fraction of the way.
  crossAt(vertical: boolean): number | null {
    if (this.tok().kind !== "At") {
      return null;
    }
    this.bump();
    const span = this.span();
    const t = this.number();
    if (t > 0.0 && t < 1.0) {
      return t;
    }
    const [kw, run] = vertical ? ["vhv", "horizontal"] : ["hvh", "vertical"];
    this.diags.push(
      Diagnostic.error(
        "parse",
        span,
        `\`${kw}@t\` needs t between 0 and 1 (exclusive), found ${displayF64(t)}`,
      ).withHelp(
        `\`${kw}@0.5\` puts the ${run} run halfway, which is the default; leave out \`@t\` for that`,
      ),
    );
    throw FAIL;
  }

  target(kw: string): Target {
    const t = this.tok();
    if (t.kind === "Num") {
      const n = t.value;
      this.bump();
      return { kind: "Num", value: n };
    }
    const p = this.point();
    const a = this.tokAt(1);
    if (this.tok().kind === "Dot" && a.kind === "Ident" && (a.value === "x" || a.value === "y")) {
      const text = this.source(pointSpan(p), pointSpan(p));
      return this.errorHelp(
        "end of statement",
        `\`${kw}\` takes a point and uses its ${kw}: write \`${kw} ${text}\``,
      );
    }
    return { kind: "Point", value: p };
  }

  curve(): SegKind {
    if (this.isKw("via")) {
      this.bump();
      const c1 = this.point();
      const c2 = this.point();
      const to = this.point();
      return { kind: "CurveVia", c1, c2, to };
    }
    let side: "Left" | "Right";
    if (this.isKw("left")) {
      side = "Left";
    } else if (this.isKw("right")) {
      side = "Right";
    } else {
      return this.error("`left <N>`, `right <N>` or `via`");
    }
    const sideSpan = this.bump();
    const colon = this.tok().kind === "Colon" ? this.bump() : null;
    const amountSpan = this.span();
    const amount = this.attempt(() => this.number());
    const to: Result<Point, null> = amount.ok
      ? this.attempt(() => this.point())
      : { ok: false, error: null };
    if (colon !== null) {
      const sideText = this.source(sideSpan, sideSpan);
      const rest = to.ok ? this.source(amountSpan, pointSpan(to.value)) : "<N> <point>";
      this.diags.push(
        Diagnostic.error("parse", colon, "`curve left` and `curve right` take no `:`").withHelp(
          `remove the \`:\`: write \`curve ${sideText} ${rest}\``,
        ),
      );
    }
    if (!amount.ok || !to.ok) {
      throw FAIL;
    }
    return { kind: "Curve", side, amount: amount.value, to: to.value };
  }
}
