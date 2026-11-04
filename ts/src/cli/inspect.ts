import { Ctx } from "../check/ctx.js";
import { bbox } from "../check/element.js";
import { corridors as freeCorridors } from "../check/free.js";
import { ports as findPorts } from "../check/ports.js";
import { type Diagnostic } from "../diag/diagnostic.js";
import { Float, type Json, Ordered } from "../diag/json.js";
import { displayF64 } from "../diag/rust.js";
import { type Pt, type Rect, length, pointAt, round2, segFrom, segTo } from "../geom/index.js";
import {
  ANCHORS,
  EdgeKind,
  Fit,
  LabelSide,
  type Scene,
  SegmentKind,
  ShapeKind,
} from "../model/index.js";
import { TOOL_VERSION } from "../model/version.js";
import { debugStr } from "../syntax/index.js";
import { plural, sha256 } from "./report.js";

export const SCHEMA = "kashika.inspect/1";

type P = [number, number];

function p(q: Pt): P {
  return [round2(q.x), round2(q.y)];
}

export interface Request {
  ids: readonly string[];
  ports: boolean;
  corridors: Rect | null;
}

interface ShapeInfo {
  id: string;
  kind: string;
  label: string | null;
  parent: string | null;
  line: number;
  bbox: [P, P];
  localBbox: [P, P];
  anchors: Map<string, P>;
  localAnchors: Map<string, P>;
  image: ImageInfo | null;
}

/// The file an image or `shape from` SVG draws.
interface ImageInfo {
  path: string;
  natural: P;
  aspect: number;
  fit: Fit;
}

interface JunctionInfo {
  id: string;
  parent: string | null;
  line: number;
  at: P;
  localAt: P;
}

interface TextInfo {
  id: string;
  content: string;
  parent: string | null;
  line: number;
  at: P;
  localAt: P;
  bbox: [P, P];
}

interface LineInfo {
  id: string;
  parent: string | null;
  line: number;
  length: number;
  vertices: P[];
  localVertices: P[];
  segments: SegInfo[];
  labels: LabelInfo[];
}

interface SegInfo {
  index: number;
  kind: SegmentKind;
  line: number;
  col: number;
  from: P;
  to: P;
}

interface LabelInfo {
  t: number;
  text: string;
  at: P;
  side: LabelSide;
  bbox: [P, P];
}

interface Edge {
  from: string;
  to: string;
  kind: EdgeKind;
}

interface PortInfo {
  shape: string;
  side: string;
  t: number;
  at: P;
  line: string;
  end: string;
}

interface Corridors {
  rect: [P, P];
  horizontal: P[];
  vertical: P[];
}

export interface Inspect {
  schema: string;
  tool: string;
  input: { name: string; sha256: string };
  counts: { error: number; warn: number };
  /// The canvas size, after any `auto` axis is fitted.
  canvas: P;
  /// Whether the text output lists everything, so it starts with the canvas.
  all: boolean;
  shapes: ShapeInfo[];
  junctions: JunctionInfo[];
  texts: TextInfo[];
  lines: LineInfo[];
  placement: Edge[];
  ports: PortInfo[] | null;
  corridors: Corridors | null;
}

/// `lints` counts the unsuppressed warnings `check` found besides those in `diags`.
export function build(
  name: string,
  src: string,
  scene: Scene,
  diags: readonly Diagnostic[],
  lints: number,
  req: Request,
): Inspect {
  const corridorsOnly = req.ids.length === 0 && req.corridors !== null;
  const keep = (id: string) => (req.ids.length === 0 ? !corridorsOnly : req.ids.includes(id));
  const errors = diags.filter((d) => d.severity === "error").length;
  const shapes = scene.shapes
    .filter((s) => keep(s.id))
    .map((s): ShapeInfo => {
      const pts: [string, Pt][] = [];
      for (const a of ANCHORS) {
        const q = s.anchor(a, null);
        if (q !== null) pts.push([a, q]);
      }
      let image: ImageInfo | null = null;
      if (s.image !== null) {
        const a = scene.assets[s.image.asset];
        if (a !== undefined) {
          const [w, h] = a.natural;
          image = {
            path: a.path,
            natural: [round2(w), round2(h)],
            aspect: h > 0.0 ? round2(w / h) : 0.0,
            fit: s.image.fit,
          };
        }
      }
      return {
        id: s.id,
        kind: ShapeKind.name(s.kind),
        label: s.label,
        parent: s.parent,
        line: s.span.line,
        bbox: bbox(s.rect),
        localBbox: bbox(s.localRect()),
        anchors: new Map(pts.map(([a, q]) => [a, p(q)])),
        localAnchors: new Map(pts.map(([a, q]) => [a, p(q.sub(s.origin))])),
        image,
      };
    });
  const junctions = scene.junctions
    .filter((j) => keep(j.id))
    .map(
      (j): JunctionInfo => ({
        id: j.id,
        parent: j.parent,
        line: j.span.line,
        at: p(j.at),
        localAt: p(j.at.sub(j.origin)),
      }),
    );
  const texts = scene.texts
    .filter((t) => keep(t.displayId()))
    .map(
      (t): TextInfo => ({
        id: t.displayId(),
        content: t.content,
        parent: t.parent,
        line: t.span.line,
        at: p(t.at),
        localAt: p(t.at.sub(t.origin)),
        bbox: bbox(t.rect),
      }),
    );
  const lines = scene.lines
    .filter((l) => keep(l.id))
    .map((l): LineInfo => {
      const vertices: Pt[] = l.path.length > 0 ? [segFrom(l.path[0]!)] : [];
      vertices.push(...l.path.map(segTo));
      return {
        id: l.id,
        parent: l.parent,
        line: l.span.line,
        length: round2(length(l.path)),
        localVertices: vertices.map((v) => p(v.sub(l.origin))),
        vertices: vertices.map(p),
        segments: l.segments.map((g, i) => ({
          index: i + 1,
          kind: g.kind,
          line: g.span.line,
          col: g.span.col,
          from: p(g.from),
          to: p(g.to),
        })),
        labels: l.labels.map((lab) => ({
          t: lab.t,
          text: lab.text,
          at: p(pointAt(l.path, lab.t)),
          side: lab.side,
          bbox: bbox(lab.rect),
        })),
      };
    });
  const placement = scene.placement
    .filter((e) => keep(e.from) || keep(e.to))
    .map((e): Edge => ({ from: e.from, to: e.to, kind: e.kind }));
  const ports = req.ports
    ? findPorts(Ctx.new(scene))
        .filter((x) => req.ids.length === 0 || keep(x.shape) || keep(x.line))
        .map(
          (x): PortInfo => ({
            shape: x.shape,
            side: x.side,
            t: round2(x.t),
            at: p(x.at),
            line: x.line,
            end: x.end,
          }),
        )
    : null;
  let corridors: Corridors | null = null;
  if (req.corridors !== null) {
    const r = req.corridors;
    const [h, v] = freeCorridors(scene, r);
    corridors = {
      rect: bbox(r),
      horizontal: h.map(([a, b]): P => [a, b]),
      vertical: v.map(([a, b]): P => [a, b]),
    };
  }
  return {
    schema: SCHEMA,
    tool: TOOL_VERSION,
    input: { name, sha256: sha256(src) },
    counts: { error: errors, warn: diags.length - errors + lints },
    canvas: [round2(scene.size[0]), round2(scene.size[1])],
    all: req.ids.length === 0 && !corridorsOnly,
    shapes,
    junctions,
    texts,
    lines,
    placement,
    ports,
    corridors,
  };
}

const f = (v: number): Json => new Float(v);
const pj = (q: P): Json => [f(q[0]), f(q[1])];
const rj = (b: [P, P]): Json => [pj(b[0]), pj(b[1])];
const anchorsJson = (m: Map<string, P>): Json => new Map([...m].map(([k, q]) => [k, pj(q)]));

/// `i` as `serde_json::to_string` writes the Rust struct: fields in declaration order.
export function toJson(i: Inspect): Json {
  const fields: [string, Json][] = [
    ["schema", i.schema],
    ["tool", i.tool],
    [
      "input",
      new Ordered([
        ["name", i.input.name],
        ["sha256", i.input.sha256],
      ]),
    ],
    [
      "counts",
      new Ordered([
        ["error", i.counts.error],
        ["warn", i.counts.warn],
      ]),
    ],
    ["canvas", pj(i.canvas)],
    [
      "shapes",
      i.shapes.map((s) => {
        const o: [string, Json][] = [
          ["id", s.id],
          ["kind", s.kind],
          ["label", s.label],
          ["parent", s.parent],
          ["line", s.line],
          ["bbox", rj(s.bbox)],
          ["local_bbox", rj(s.localBbox)],
          ["anchors", anchorsJson(s.anchors)],
          ["local_anchors", anchorsJson(s.localAnchors)],
        ];
        if (s.image !== null) {
          o.push([
            "image",
            new Ordered([
              ["path", s.image.path],
              ["natural", pj(s.image.natural)],
              ["aspect", f(s.image.aspect)],
              ["fit", Fit.name(s.image.fit)],
            ]),
          ]);
        }
        return new Ordered(o);
      }),
    ],
    [
      "junctions",
      i.junctions.map(
        (j) =>
          new Ordered([
            ["id", j.id],
            ["parent", j.parent],
            ["line", j.line],
            ["at", pj(j.at)],
            ["local_at", pj(j.localAt)],
          ]),
      ),
    ],
    [
      "texts",
      i.texts.map(
        (t) =>
          new Ordered([
            ["id", t.id],
            ["content", t.content],
            ["parent", t.parent],
            ["line", t.line],
            ["at", pj(t.at)],
            ["local_at", pj(t.localAt)],
            ["bbox", rj(t.bbox)],
          ]),
      ),
    ],
    [
      "lines",
      i.lines.map(
        (l) =>
          new Ordered([
            ["id", l.id],
            ["parent", l.parent],
            ["line", l.line],
            ["length", f(l.length)],
            ["vertices", l.vertices.map(pj)],
            ["local_vertices", l.localVertices.map(pj)],
            [
              "segments",
              l.segments.map(
                (s) =>
                  new Ordered([
                    ["index", s.index],
                    ["kind", SegmentKind.name(s.kind)],
                    ["line", s.line],
                    ["col", s.col],
                    ["from", pj(s.from)],
                    ["to", pj(s.to)],
                  ]),
              ),
            ],
            [
              "labels",
              l.labels.map((lab) => {
                const o: [string, Json][] = [
                  ["t", f(lab.t)],
                  ["text", lab.text],
                  ["at", pj(lab.at)],
                ];
                if (!LabelSide.isOn(lab.side)) o.push(["side", LabelSide.name(lab.side)]);
                o.push(["bbox", rj(lab.bbox)]);
                return new Ordered(o);
              }),
            ],
          ]),
      ),
    ],
    [
      "placement",
      i.placement.map(
        (e) =>
          new Ordered([
            ["from", e.from],
            ["to", e.to],
            ["kind", EdgeKind.name(e.kind)],
          ]),
      ),
    ],
  ];
  if (i.ports !== null) {
    fields.push([
      "ports",
      i.ports.map(
        (x) =>
          new Ordered([
            ["shape", x.shape],
            ["side", x.side],
            ["t", f(x.t)],
            ["at", pj(x.at)],
            ["line", x.line],
            ["end", x.end],
          ]),
      ),
    ]);
  }
  if (i.corridors !== null) {
    fields.push([
      "corridors",
      new Ordered([
        ["rect", rj(i.corridors.rect)],
        ["horizontal", i.corridors.horizontal.map(pj)],
        ["vertical", i.corridors.vertical.map(pj)],
      ]),
    ]);
  }
  return new Ordered(fields);
}

function pt(q: P): string {
  return `(${displayF64(q[0])},${displayF64(q[1])})`;
}

function pts(qs: readonly P[]): string {
  return qs.map(pt).join(" ");
}

function rect(b: [P, P]): string {
  return `${pt(b[0])}-${pt(b[1])}`;
}

function anchors(m: Map<string, P>): string {
  return ANCHORS.filter((a) => m.has(a))
    .map((a) => `${a} ${pt(m.get(a)!)}`)
    .join(" ");
}

function intervals(v: readonly P[]): string {
  if (v.length === 0) {
    return "none";
  }
  return v.map((i) => `${displayF64(i[0])}..${displayF64(i[1])}`).join(", ");
}

function within(parent: string | null): string {
  return parent === null ? "" : ` in ${parent}`;
}

export function text(i: Inspect): string {
  let out = "";
  if (i.all) {
    out += `canvas ${displayF64(i.canvas[0])}x${displayF64(i.canvas[1])}\n`;
  }
  for (const s of i.shapes) {
    const label = s.label === null ? "" : ` ${debugStr(s.label)}`;
    out += `${s.kind} ${s.id}${label} (line ${s.line})${within(s.parent)}\n`;
    out += `  bbox ${rect(s.bbox)}\n`;
    out += `  anchors ${anchors(s.anchors)}\n`;
    if (s.image !== null) {
      const img = s.image;
      out += `  image ${debugStr(img.path)} natural ${displayF64(img.natural[0])}x${displayF64(img.natural[1])} aspect ${displayF64(img.aspect)} fit=${Fit.name(img.fit)}\n`;
    }
    if (s.parent !== null) {
      out += `  local bbox ${rect(s.localBbox)}\n`;
      out += `  local anchors ${anchors(s.localAnchors)}\n`;
    }
  }
  for (const j of i.junctions) {
    out += `junction ${j.id} at ${pt(j.at)} (line ${j.line})${within(j.parent)}\n`;
    if (j.parent !== null) {
      out += `  local ${pt(j.localAt)}\n`;
    }
  }
  for (const t of i.texts) {
    out += `text ${t.id} ${debugStr(t.content)} at ${pt(t.at)} bbox ${rect(t.bbox)} (line ${t.line})${within(t.parent)}\n`;
    if (t.parent !== null) {
      out += `  local ${pt(t.localAt)}\n`;
    }
  }
  for (const l of i.lines) {
    out += `line ${l.id} (line ${l.line})${within(l.parent)} length ${displayF64(l.length)}px\n`;
    out += `  vertices ${pts(l.vertices)}\n`;
    if (l.parent !== null) {
      out += `  local vertices ${pts(l.localVertices)}\n`;
    }
    for (const s of l.segments) {
      out += `  segment ${s.index} ${SegmentKind.name(s.kind)} (${s.line}:${s.col}) ${pt(s.from)} -> ${pt(s.to)}\n`;
    }
    for (const lab of l.labels) {
      const side = LabelSide.isOn(lab.side) ? "" : ` ${LabelSide.name(lab.side)}`;
      out += `  label@${displayF64(lab.t)} ${debugStr(lab.text)} at ${pt(lab.at)}${side} bbox ${rect(lab.bbox)}\n`;
    }
  }
  if (i.placement.length > 0) {
    out += "placement\n";
    for (const e of i.placement) {
      out += `  ${e.from} -> ${e.to} (${EdgeKind.name(e.kind)})\n`;
    }
  }
  if (i.ports !== null) {
    out += "ports\n";
    for (const x of i.ports) {
      const anchor =
        x.t === 0.5 ? `${x.shape}.${x.side}` : `${x.shape}.${x.side}@${displayF64(x.t)}`;
      out += `  ${anchor} ${pt(x.at)}: ${x.line} ${x.end}\n`;
    }
  }
  if (i.corridors !== null) {
    const c = i.corridors;
    out += `corridors ${rect(c.rect)}\n`;
    out += `  horizontal, free y: ${intervals(c.horizontal)}\n`;
    out += `  vertical, free x: ${intervals(c.vertical)}\n`;
  }
  out += `${plural(i.counts.error, "error")}, ${plural(i.counts.warn, "warning")}\n`;
  return out;
}
