import type { Span } from "../diag/index.js";

// Field names are the Rust field names, in snake_case and in the Rust order, which is what
// `debug.ts` prints. A data enum is a union tagged by `kind`, the Rust variant name; a tuple
// variant keeps its payload in `value` (a tuple for two fields). A unit enum is a union of
// its variant names.

export interface File {
  stmts: Stmt[];
  /// Where the first statement starts, whether or not it parsed.
  first_stmt: Span | null;
  /// Ids declared by statements that failed to parse after their id was read, and by the
  /// children of a group that failed to parse; the parse error already explains why
  /// references to them don't resolve.
  failed_ids: Ident[];
  /// Whether a `diagram` statement failed to parse.
  failed_diagram: boolean;
}

export type Stmt =
  | { kind: "Diagram"; value: Diagram }
  | { kind: "Style"; value: StyleDef }
  | { kind: "Box"; value: Node }
  | { kind: "Group"; value: Group }
  | { kind: "Junction"; value: Junction }
  | { kind: "Text"; value: Text }
  | { kind: "Line"; value: Line }
  | { kind: "Import"; value: Import }
  | { kind: "Shape"; value: ShapeDef }
  | { kind: "Image"; value: Image };

export function stmtSpan(s: Stmt): Span {
  switch (s.kind) {
    case "Diagram":
      return s.value.span;
    case "Style":
      return s.value.span;
    case "Box":
      return s.value.span;
    case "Group":
      return s.value.node.span;
    case "Junction":
      return s.value.span;
    case "Text":
      return s.value.span;
    case "Line":
      return s.value.span;
    case "Import":
      return s.value.span;
    case "Shape":
      return s.value.span;
    case "Image":
      return s.value.span;
  }
}

export interface Diagram {
  title: string;
  /// `null` when omitted; the canvas then fits what is drawn.
  size: Size | null;
  attrs: Attr[];
  span: Span;
}

export interface StyleDef {
  name: string;
  attrs: Attr[];
  span: Span;
}

/// A `box`, or the header of a `group`.
export interface Node {
  id: Ident;
  label: string | null;
  at: Point;
  /// `null` when omitted; a box then fits its text.
  size: Size | null;
  attrs: Attr[];
  /// A box's `{ }` text block; always `null` on a group header.
  block: BlockText[] | null;
  span: Span;
}

/// A `text` item inside a box's text block. `at` is in the box-local frame.
export interface BlockText {
  content: string;
  at: Point | null;
  attrs: Attr[];
  span: Span;
}

export interface Group {
  node: Node;
  children: Stmt[];
}

export interface Junction {
  id: Ident;
  at: Point;
  attrs: Attr[];
  span: Span;
}

export interface Text {
  id: Ident | null;
  content: string;
  at: Point;
  attrs: Attr[];
  span: Span;
}

export interface Line {
  id: Ident | null;
  /// Written as an `arrow` statement, whose `end` defaults to `arrow`.
  arrow: boolean;
  start: Point;
  segs: Seg[];
  attrs: Attr[];
  span: Span;
}

export interface Import {
  path: string;
  path_span: Span;
  /// The `as <ns>` namespace.
  ns: Ident | null;
  span: Span;
}

export interface ShapeDef {
  name: Ident;
  body: ShapeBody;
  span: Span;
}

export type Quad = [number, number, number, number];

export type ShapeBody =
  /// `shape <name> { path "..." [outline N] [text-area (x,y,w,h)] }`; each value keeps its span.
  | {
      kind: "Paths";
      path: [string, Span];
      outline: [number, Span] | null;
      text_area: [Quad, Span] | null;
    }
  /// `shape <name> from "<file.svg>" [attrs]`.
  | { kind: "From"; file: string; file_span: Span; attrs: Attr[] };

export interface Image {
  id: Ident;
  file: string;
  file_span: Span;
  at: Point;
  size: [number, number];
  attrs: Attr[];
  span: Span;
}

/// A `size (w,h)` clause; an axis written `auto` is `null`.
export interface Size {
  w: number | null;
  h: number | null;
}

export interface Ident {
  name: string;
  span: Span;
}

export interface Attr {
  key: Ident;
  /// The `t` of `label@t=...`.
  at_t: number | null;
  value: Value;
  span: Span;
}

export type Value =
  | { kind: "Flag" }
  | { kind: "Num"; value: number }
  | { kind: "Str"; value: string }
  | { kind: "Color"; value: string }
  /// An identifier, or a `:`/`,`/`.`-joined list of them, e.g. `cross:l2,near` or `aws.s3`.
  | { kind: "Word"; value: string }
  | { kind: "Pair"; value: [number, number] }
  /// `size=(w,h)`, where an axis may be `auto`.
  | { kind: "Size"; value: Size }
  /// `side@t`, accepted only as the value of `pin`.
  | { kind: "AnchorAt"; value: [string, number] };

export type Point =
  | { kind: "Lit"; x: Coord; y: Coord; span: Span }
  | { kind: "Ref"; r: Ref; offset: Offset | null; span: Span }
  /// `(P)`: parentheses around a point, which mean `P`.
  | { kind: "Group"; inner: Point; span: Span }
  /// A point followed by offsets, which add up. A reference's first offset stays on the
  /// `Ref`, so this holds the rest.
  | { kind: "Shifted"; base: Point; offsets: Offset[]; span: Span };

/// `+(dx,dy)` or `-(dx,dy)`; `neg` records which sign was written.
export interface Offset {
  neg: boolean;
  dx: number;
  dy: number;
}

/// The offset with its sign applied.
export function offsetDelta(o: Offset): [number, number] {
  if (o.neg) {
    return [-o.dx, -o.dy];
  } else {
    return [o.dx, o.dy];
  }
}

export function pointSpan(p: Point): Span {
  return p.span;
}

/// The point without its grouping parentheses and the offsets that follow them: the
/// reference, or the literal or mixed point, that the point is relative to.
export function pointBase(p: Point): Point {
  switch (p.kind) {
    case "Group":
      return pointBase(p.inner);
    case "Shifted":
      return pointBase(p.base);
    default:
      return p;
  }
}

/// The point without its grouping parentheses.
export function pointUngrouped(p: Point): Point {
  return p.kind === "Group" ? pointUngrouped(p.inner) : p;
}

export type Coord = { kind: "Num"; value: number } | { kind: "Axis"; r: Ref; axis: Axis };

export type Axis = "X" | "Y";

export interface Ref {
  id: Ident;
  anchor: Anchor | null;
}

export interface Anchor {
  name: Ident;
  t: number | null;
}

export interface Seg {
  kind: SegKind;
  span: Span;
}

export type SegKind =
  | { kind: "To"; value: Point }
  | { kind: "H"; value: number }
  | { kind: "V"; value: number }
  | { kind: "X"; value: Target }
  | { kind: "Y"; value: Target }
  | { kind: "Hv"; value: Point }
  | { kind: "Vh"; value: Point }
  /// `vhv[@t] <point>`: vertical, horizontal at `t` of the way (default 0.5), vertical.
  | { kind: "Vhv"; t: number | null; to: Point }
  /// `hvh[@t] <point>`: horizontal, vertical at `t` of the way (default 0.5), horizontal.
  | { kind: "Hvh"; t: number | null; to: Point }
  | { kind: "Curve"; side: Side; amount: number; to: Point }
  | { kind: "CurveVia"; c1: Point; c2: Point; to: Point };

export type Target = { kind: "Num"; value: number } | { kind: "Point"; value: Point };

export type Side = "Left" | "Right";
