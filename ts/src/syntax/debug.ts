import { type Span, debugF64 } from "../diag/index.js";
import type {
  Anchor,
  Attr,
  BlockText,
  Coord,
  File,
  Ident,
  Node,
  Offset,
  Point,
  Ref,
  Seg,
  SegKind,
  ShapeBody,
  Size,
  Stmt,
  Target,
  Value,
} from "./ast.js";

/// A value as Rust's derived `Debug` sees it.
type D =
  | { t: "leaf"; s: string }
  | { t: "struct"; name: string; fields: [string, D][] }
  | { t: "tuple"; name: string; items: D[] }
  | { t: "list"; items: D[] };

const leaf = (s: string): D => ({ t: "leaf", s });
const struct = (name: string, fields: [string, D][]): D => ({ t: "struct", name, fields });
const tuple = (name: string, ...items: D[]): D => ({ t: "tuple", name, items });
const list = (items: D[]): D => ({ t: "list", items });
const f64 = (v: number): D => leaf(debugF64(v));
const bool = (v: boolean): D => leaf(v ? "true" : "false");
const str = (s: string): D => leaf(debugStr(s));
const opt = <T>(v: T | null, f: (v: T) => D): D => (v === null ? leaf("None") : tuple("Some", f(v)));

/// Rust's `{:#?}` text of the AST.
export function debugFile(file: File): string {
  return render(fileD(file));
}

function render(d: D): string {
  switch (d.t) {
    case "leaf":
      return d.s;
    case "struct":
      return `${d.name} {\n${d.fields.map(([k, v]) => `    ${k}: ${indent(render(v))},\n`).join("")}}`;
    case "tuple":
      return `${d.name}(\n${d.items.map((v) => `    ${indent(render(v))},\n`).join("")})`;
    case "list":
      if (d.items.length === 0) return "[]";
      return `[\n${d.items.map((v) => `    ${indent(render(v))},\n`).join("")}]`;
  }
}

function indent(s: string): string {
  return s.replaceAll("\n", "\n    ");
}

const GRAPHEME_EXTEND = /^\p{Grapheme_Extend}$/u;
const NOT_PRINTABLE = /^[\p{Cc}\p{Cf}\p{Cs}\p{Co}\p{Cn}\p{Zl}\p{Zp}\p{Zs}]$/u;

/// Rust's `{:?}` for `str`: `char::escape_debug` with grapheme extenders escaped and `'` kept.
export function debugStr(s: string): string {
  let out = '"';
  for (const c of s) {
    switch (c) {
      case "\0":
        out += "\\0";
        break;
      case "\t":
        out += "\\t";
        break;
      case "\r":
        out += "\\r";
        break;
      case "\n":
        out += "\\n";
        break;
      case "\\":
        out += "\\\\";
        break;
      case '"':
        out += '\\"';
        break;
      default:
        if (GRAPHEME_EXTEND.test(c) || (c !== " " && NOT_PRINTABLE.test(c))) {
          out += `\\u{${c.codePointAt(0)!.toString(16)}}`;
        } else {
          out += c;
        }
    }
  }
  return out + '"';
}

function spanD(s: Span): D {
  return struct("Span", [
    ["line", leaf(String(s.line))],
    ["col", leaf(String(s.col))],
    ["end_line", leaf(String(s.endLine))],
    ["end_col", leaf(String(s.endCol))],
  ]);
}

function fileD(f: File): D {
  return struct("File", [
    ["stmts", list(f.stmts.map(stmtD))],
    ["first_stmt", opt(f.first_stmt, spanD)],
    ["failed_ids", list(f.failed_ids.map(identD))],
    ["failed_diagram", bool(f.failed_diagram)],
  ]);
}

function stmtD(s: Stmt): D {
  switch (s.kind) {
    case "Diagram": {
      const d = s.value;
      return tuple(
        "Diagram",
        struct("Diagram", [
          ["title", str(d.title)],
          ["size", opt(d.size, sizeD)],
          ["attrs", attrsD(d.attrs)],
          ["span", spanD(d.span)],
        ]),
      );
    }
    case "Style": {
      const d = s.value;
      return tuple(
        "Style",
        struct("StyleDef", [
          ["name", str(d.name)],
          ["attrs", attrsD(d.attrs)],
          ["span", spanD(d.span)],
        ]),
      );
    }
    case "Box":
      return tuple("Box", nodeD(s.value));
    case "Group":
      return tuple(
        "Group",
        struct("Group", [
          ["node", nodeD(s.value.node)],
          ["children", list(s.value.children.map(stmtD))],
        ]),
      );
    case "Junction": {
      const j = s.value;
      return tuple(
        "Junction",
        struct("Junction", [
          ["id", identD(j.id)],
          ["at", pointD(j.at)],
          ["attrs", attrsD(j.attrs)],
          ["span", spanD(j.span)],
        ]),
      );
    }
    case "Text": {
      const t = s.value;
      return tuple(
        "Text",
        struct("Text", [
          ["id", opt(t.id, identD)],
          ["content", str(t.content)],
          ["at", pointD(t.at)],
          ["attrs", attrsD(t.attrs)],
          ["span", spanD(t.span)],
        ]),
      );
    }
    case "Line": {
      const l = s.value;
      return tuple(
        "Line",
        struct("Line", [
          ["id", opt(l.id, identD)],
          ["arrow", bool(l.arrow)],
          ["start", pointD(l.start)],
          ["segs", list(l.segs.map(segD))],
          ["attrs", attrsD(l.attrs)],
          ["span", spanD(l.span)],
        ]),
      );
    }
    case "Import": {
      const i = s.value;
      return tuple(
        "Import",
        struct("Import", [
          ["path", str(i.path)],
          ["path_span", spanD(i.path_span)],
          ["ns", opt(i.ns, identD)],
          ["span", spanD(i.span)],
        ]),
      );
    }
    case "Shape": {
      const d = s.value;
      return tuple(
        "Shape",
        struct("ShapeDef", [
          ["name", identD(d.name)],
          ["body", shapeBodyD(d.body)],
          ["span", spanD(d.span)],
        ]),
      );
    }
    case "Image": {
      const i = s.value;
      return tuple(
        "Image",
        struct("Image", [
          ["id", identD(i.id)],
          ["file", str(i.file)],
          ["file_span", spanD(i.file_span)],
          ["at", pointD(i.at)],
          ["size", tuple("", f64(i.size[0]), f64(i.size[1]))],
          ["attrs", attrsD(i.attrs)],
          ["span", spanD(i.span)],
        ]),
      );
    }
  }
}

function nodeD(n: Node): D {
  return struct("Node", [
    ["id", identD(n.id)],
    ["label", opt(n.label, str)],
    ["at", pointD(n.at)],
    ["size", opt(n.size, sizeD)],
    ["attrs", attrsD(n.attrs)],
    ["block", opt(n.block, (b) => list(b.map(blockTextD)))],
    ["span", spanD(n.span)],
  ]);
}

function blockTextD(b: BlockText): D {
  return struct("BlockText", [
    ["content", str(b.content)],
    ["at", opt(b.at, pointD)],
    ["attrs", attrsD(b.attrs)],
    ["span", spanD(b.span)],
  ]);
}

function shapeBodyD(b: ShapeBody): D {
  switch (b.kind) {
    case "Paths":
      return struct("Paths", [
        ["path", tuple("", str(b.path[0]), spanD(b.path[1]))],
        ["outline", opt(b.outline, ([n, s]) => tuple("", f64(n), spanD(s)))],
        ["text_area", opt(b.text_area, ([q, s]) => tuple("", list(q.map(f64)), spanD(s)))],
      ]);
    case "From":
      return struct("From", [
        ["file", str(b.file)],
        ["file_span", spanD(b.file_span)],
        ["attrs", attrsD(b.attrs)],
      ]);
  }
}

function sizeD(s: Size): D {
  return struct("Size", [
    ["w", opt(s.w, f64)],
    ["h", opt(s.h, f64)],
  ]);
}

function identD(i: Ident): D {
  return struct("Ident", [
    ["name", str(i.name)],
    ["span", spanD(i.span)],
  ]);
}

function attrsD(attrs: Attr[]): D {
  return list(attrs.map(attrD));
}

function attrD(a: Attr): D {
  return struct("Attr", [
    ["key", identD(a.key)],
    ["at_t", opt(a.at_t, f64)],
    ["value", valueD(a.value)],
    ["span", spanD(a.span)],
  ]);
}

function valueD(v: Value): D {
  switch (v.kind) {
    case "Flag":
      return leaf("Flag");
    case "Num":
      return tuple("Num", f64(v.value));
    case "Str":
    case "Color":
    case "Word":
      return tuple(v.kind, str(v.value));
    case "Pair":
      return tuple("Pair", f64(v.value[0]), f64(v.value[1]));
    case "Size":
      return tuple("Size", sizeD(v.value));
    case "AnchorAt":
      return tuple("AnchorAt", str(v.value[0]), f64(v.value[1]));
  }
}

function pointD(p: Point): D {
  switch (p.kind) {
    case "Lit":
      return struct("Lit", [
        ["x", coordD(p.x)],
        ["y", coordD(p.y)],
        ["span", spanD(p.span)],
      ]);
    case "Ref":
      return struct("Ref", [
        ["r", refD(p.r)],
        ["offset", opt(p.offset, offsetD)],
        ["span", spanD(p.span)],
      ]);
    case "Group":
      return struct("Group", [
        ["inner", pointD(p.inner)],
        ["span", spanD(p.span)],
      ]);
    case "Shifted":
      return struct("Shifted", [
        ["base", pointD(p.base)],
        ["offsets", list(p.offsets.map(offsetD))],
        ["span", spanD(p.span)],
      ]);
  }
}

function offsetD(o: Offset): D {
  return struct("Offset", [
    ["neg", bool(o.neg)],
    ["dx", f64(o.dx)],
    ["dy", f64(o.dy)],
  ]);
}

function coordD(c: Coord): D {
  switch (c.kind) {
    case "Num":
      return tuple("Num", f64(c.value));
    case "Axis":
      return struct("Axis", [
        ["r", refD(c.r)],
        ["axis", leaf(c.axis)],
      ]);
  }
}

function refD(r: Ref): D {
  return struct("Ref", [
    ["id", identD(r.id)],
    ["anchor", opt(r.anchor, anchorD)],
  ]);
}

function anchorD(a: Anchor): D {
  return struct("Anchor", [
    ["name", identD(a.name)],
    ["t", opt(a.t, f64)],
  ]);
}

function segD(s: Seg): D {
  return struct("Seg", [
    ["kind", segKindD(s.kind)],
    ["span", spanD(s.span)],
  ]);
}

function segKindD(k: SegKind): D {
  switch (k.kind) {
    case "To":
    case "Hv":
    case "Vh":
      return tuple(k.kind, pointD(k.value));
    case "H":
    case "V":
      return tuple(k.kind, f64(k.value));
    case "X":
    case "Y":
      return tuple(k.kind, targetD(k.value));
    case "Vhv":
    case "Hvh":
      return struct(k.kind, [
        ["t", opt(k.t, f64)],
        ["to", pointD(k.to)],
      ]);
    case "Curve":
      return struct("Curve", [
        ["side", leaf(k.side)],
        ["amount", f64(k.amount)],
        ["to", pointD(k.to)],
      ]);
    case "CurveVia":
      return struct("CurveVia", [
        ["c1", pointD(k.c1)],
        ["c2", pointD(k.c2)],
        ["to", pointD(k.to)],
      ]);
  }
}

function targetD(t: Target): D {
  switch (t.kind) {
    case "Num":
      return tuple("Num", f64(t.value));
    case "Point":
      return tuple("Point", pointD(t.value));
  }
}
