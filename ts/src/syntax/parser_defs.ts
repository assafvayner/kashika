import { Diagnostic, Span, didYouMean } from "../diag/index.js";
import type { Ident, Image, Import, Quad, ShapeDef } from "./ast.js";
import { Clauses, FAIL, type Parser } from "./parser.js";

const SHAPE_KEYS: readonly string[] = ["path", "outline", "text-area"];

type PathBody = [[string, Span] | null, [number, Span] | null, [Quad, Span] | null];

interface Slot<T> {
  v: T | null;
}

export function importStmt(p: Parser, start: Span): Import {
  const pathSpan = p.span();
  const path = p.string();
  let ns: Ident | null = null;
  if (p.isKw("as")) {
    p.bump();
    ns = p.ident("a namespace name");
  }
  return { path, path_span: pathSpan, ns, span: start.to(p.prevSpan()) };
}

export function image(p: Parser, start: Span): Image {
  const id = p.declId("an id");
  const fileSpan = p.span();
  const t = p.tok();
  if (t.kind !== "Str") {
    return p.error("a file path string");
  }
  const file = t.value;
  p.bump();
  const tail = p.tail(Clauses.AT_SIZE);
  return {
    id,
    file,
    file_span: fileSpan,
    at: p.required(tail.at, "at"),
    size: p.fixedSize(tail.size, "an image's"),
    attrs: tail.attrs,
    span: start.to(p.prevSpan()),
  };
}

export function shapeDef(p: Parser, start: Span): ShapeDef {
  const name = p.ident("a shape name");
  if (p.isKw("from")) {
    p.bump();
    const fileSpan = p.span();
    const file = p.string();
    const attrs = p.attrs([]);
    return {
      name,
      body: { kind: "From", file, file_span: fileSpan, attrs },
      span: start.to(p.prevSpan()),
    };
  }
  p.expect({ kind: "LBrace" });
  const body = p.attempt(() => shapeBody(p));
  if (!body.ok) {
    skipBlock(p);
    throw FAIL;
  }
  const [path, outline, textArea] = body.value;
  const span = start.to(p.prevSpan());
  if (path === null) {
    p.diags.push(
      Diagnostic.error("parse", span, `shape \`${name.name}\` needs a \`path\``).withHelp(
        'e.g. `path "M 0 0 L 1 0 L 1 1 L 0 1 Z"`',
      ),
    );
    throw FAIL;
  }
  return {
    name,
    body: { kind: "Paths", path, outline, text_area: textArea },
    span,
  };
}

/// Parses the items of a `shape { }` body up to and including the closing `}`.
function shapeBody(p: Parser): PathBody {
  const path: Slot<[string, Span]> = { v: null };
  const outline: Slot<[number, Span]> = { v: null };
  const textArea: Slot<[Quad, Span]> = { v: null };
  for (;;) {
    p.skipNewlines();
    const k = p.tok().kind;
    if (k === "RBrace") {
      p.bump();
      return [path.v, outline.v, textArea.v];
    }
    if (k === "Eof") {
      return p.error("`}`");
    }
    const key = p.ident("`path`, `outline` or `text-area`");
    const vstart = p.span();
    switch (key.name) {
      case "path": {
        const s = p.string();
        const span = vstart.to(p.prevSpan());
        set(path, [s, span], key, p.diags);
        break;
      }
      case "outline": {
        const n = p.number();
        const span = vstart.to(p.prevSpan());
        set(outline, [n, span], key, p.diags);
        break;
      }
      case "text-area": {
        const q = quad(p);
        const span = vstart.to(p.prevSpan());
        set(textArea, [q, span], key, p.diags);
        break;
      }
      default: {
        const other = key.name;
        let d = Diagnostic.error(
          "parse",
          key.span,
          `expected \`path\`, \`outline\` or \`text-area\` in a shape body, found \`${other}\``,
        );
        const s = didYouMean(other, SHAPE_KEYS);
        if (s !== null) {
          d = d.withHelp(`did you mean \`${s}\`?`);
        }
        p.diags.push(d);
        throw FAIL;
      }
    }
    const k2 = p.tok().kind;
    if (!(k2 === "Newline" || k2 === "RBrace")) {
      return p.error("end of line");
    }
  }
}

function quad(p: Parser): Quad {
  p.expect({ kind: "LParen" });
  const v: Quad = [0.0, 0.0, 0.0, 0.0];
  for (let i = 0; i < 4; i++) {
    if (i > 0) {
      p.expect({ kind: "Comma" });
    }
    v[i] = p.number();
  }
  p.expect({ kind: "RParen" });
  return v;
}

/// Skips to just past the `}` that closes the block whose `{` was already consumed.
function skipBlock(p: Parser): void {
  let depth = 1;
  for (;;) {
    const k = p.tok().kind;
    if (k === "Eof") return;
    if (k === "LBrace") {
      depth += 1;
    } else if (k === "RBrace") {
      depth -= 1;
      if (depth === 0) {
        p.bump();
        return;
      }
    }
    p.bump();
  }
}

function set<T>(slot: Slot<T>, v: T, key: Ident, diags: Diagnostic[]): void {
  if (slot.v !== null) {
    diags.push(Diagnostic.error("parse", key.span, `duplicate \`${key.name}\` in shape body`));
  } else {
    slot.v = v;
  }
}
