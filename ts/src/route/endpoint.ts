import { type Diagnostic } from "../diag/diagnostic.js";
import { type Json } from "../diag/json.js";
import { type Result, displayF64, err, lines, ok } from "../diag/rust.js";
import { Pt, segFrom, segTo } from "../geom/index.js";
import { type Options, compileWith } from "../model/index.js";
import { type ast, offsetDelta, parse } from "../syntax/index.js";
import { type RouteError } from "./index.js";
import { Source } from "./source.js";

/// Id prefix of the probe lines that route adds to the source to resolve and score paths.
export const PROBE = "kashika_route_probe";

export type Side = "top" | "bottom" | "left" | "right";

export const Side = {
  /// Unit vector pointing out of the shape.
  outward(s: Side): Pt {
    switch (s) {
      case "top":
        return new Pt(0.0, -1.0);
      case "bottom":
        return new Pt(0.0, 1.0);
      case "left":
        return new Pt(-1.0, 0.0);
      case "right":
        return new Pt(1.0, 0.0);
    }
  },

  of(anchor: string): Side | null {
    switch (anchor) {
      case "top":
      case "bottom":
      case "left":
      case "right":
        return anchor;
      default:
        return null;
    }
  },
};

export interface EndpointFields {
  /// Source text of a reference such as `pg.left@0.25` or a mixed point such as
  /// `(db.left.x, 120)`; `null` for literal points.
  expr: string | null;
  /// Canvas position.
  at: Pt;
  /// The id the reference names.
  owner: string | null;
  /// The side of `owner` the anchor is on, for `top`/`bottom`/`left`/`right` anchors.
  side: Side | null;
  /// A reference split into its text without the offset, and the offset. Not serialized.
  anchor: [string, Pt] | null;
}

export interface Endpoint extends EndpointFields {}

export class Endpoint {
  constructor(f: EndpointFields) {
    Object.assign(this, f);
  }

  /// An endpoint written as a literal, or that no point names.
  static literal(at: Pt): Endpoint {
    return new Endpoint({ expr: null, at, owner: null, side: null, anchor: null });
  }

  /// The endpoint a point of a `line` statement resolves to: `text` is how it is written and
  /// `at` where it lands.
  static of(p: ast.Point, text: string, at: Pt): Endpoint {
    return Endpoint.shifted(p, text, at, new Pt(0.0, 0.0));
  }

  /// As `Endpoint.of`, for a point written with offsets that add up to `shift` after it.
  static shifted(p: ast.Point, text: string, at: Pt, shift: Pt): Endpoint {
    switch (p.kind) {
      case "Ref": {
        const [dx, dy] = p.offset === null ? [0.0, 0.0] : offsetDelta(p.offset);
        return new Endpoint({
          expr: text,
          at,
          owner: p.r.id.name,
          side: p.r.anchor === null ? null : Side.of(p.r.anchor.name.name),
          anchor: [refText(p.r), shift.add(new Pt(dx, dy))],
        });
      }
      case "Lit": {
        const mixed = [p.x, p.y].some((c) => c.kind === "Axis");
        const e = Endpoint.literal(at);
        e.expr = mixed ? text : null;
        return e;
      }
      case "Group":
        return Endpoint.shifted(p.inner, text, at, shift);
      case "Shifted": {
        const s = p.offsets.reduce((s, o) => {
          const [dx, dy] = offsetDelta(o);
          return s.add(new Pt(dx, dy));
        }, shift);
        return Endpoint.shifted(p.base, text, at, s);
      }
    }
  }

  toJson(): Json {
    return { expr: this.expr, at: this.at.toJson(), owner: this.owner, side: this.side };
  }
}

/// A reference as written without its offset: `db`, `db.left` or `db.left@0.25`.
function refText(r: ast.Ref): string {
  let s = r.id.name;
  if (r.anchor !== null) {
    s += ".";
    s += r.anchor.name.name;
    if (r.anchor.t !== null) {
      s += `@${displayF64(r.anchor.t)}`;
    }
  }
  return s;
}

/// The point a line's last segment ends on, when that segment names one.
export function endPoint(line: ast.Line): ast.Point | null {
  const last = line.segs[line.segs.length - 1];
  if (last === undefined) return null;
  const k = last.kind;
  switch (k.kind) {
    case "To":
    case "Hv":
    case "Vh":
      return k.value;
    case "Vhv":
    case "Hvh":
    case "Curve":
    case "CurveVia":
      return k.to;
    case "H":
    case "V":
    case "X":
    case "Y":
      return null;
  }
}

/// Resolves two points, written as in a `.kash` file, the way a `line <from> -> <to>`
/// statement appended to `src` would. Literal points are in the canvas frame.
export function resolveEndpoints(
  src: string,
  from: string,
  to: string,
  opts: Options,
): Result<[Endpoint, Endpoint], RouteError> {
  const stmt = `line ${PROBE} ${from} -> ${to}`;
  const firstProbeLine = lines(src).length + 1;
  const [scene, diags] = compileWith(`${src}\n${stmt}\n`, opts);
  const errors: Diagnostic[] = diags.filter(
    (d) => d.severity === "error" && d.span.line >= firstProbeLine && d.code !== "not-ortho",
  );
  if (errors.length > 0) {
    return err({ kind: "Diagnostics", diags: errors });
  }
  const unresolved = (): RouteError => ({
    kind: "Usage",
    message: `cannot resolve \`${from}\` -> \`${to}\``,
  });
  const { file } = parse(stmt);
  let points: [ast.Point, ast.Point] | null = null;
  const only = file.stmts.length === 1 ? file.stmts[0]! : null;
  if (only !== null && only.kind === "Line") {
    const seg = only.value.segs.length === 1 ? only.value.segs[0]!.kind : null;
    if (seg !== null && seg.kind === "To") {
      points = [only.value.start, seg.value];
    }
  }
  if (points === null) {
    return err({
      kind: "Usage",
      message: `\`<from>\` and \`<to>\` must each be one point, not \`${from}\` -> \`${to}\``,
    });
  }
  const [start, end] = points;
  const line = scene.lines.find((l) => l.id === PROBE);
  if (line === undefined) {
    return err(unresolved());
  }
  const a = line.path[0];
  const b = line.path[line.path.length - 1];
  if (a === undefined || b === undefined) {
    return err(unresolved());
  }
  const text = Source.new(stmt);
  return ok([
    Endpoint.of(start, text.slice(start.span), segFrom(a)),
    Endpoint.of(end, text.slice(end.span), segTo(b)),
  ]);
}
