import { Ctx } from "../check/ctx.js";
import { type Axis } from "../check/free.js";
import { type Diagnostic } from "../diag/diagnostic.js";
import { Float, type Json } from "../diag/json.js";
import { type Result, cmpTuple, err, ok, round, totalCmp } from "../diag/rust.js";
import {
  type PathSeg,
  Pt,
  endDir,
  flatten,
  fmtNum,
  isAxisAligned,
  segFrom,
  segTo,
  startDir,
} from "../geom/index.js";
import { type Line, type Options, Scene, compileWith, resolveWith } from "../model/index.js";
import { type ast, parse, pointBase } from "../syntax/index.js";
import { type LaneAnchors, type Plan, corners, generate, key, laneAnchors } from "./candidates.js";
import { Endpoint, PROBE, Side, endPoint, resolveEndpoints } from "./endpoint.js";
import * as score from "./score.js";
import { Source } from "./source.js";
import { pathText, statement } from "./text.js";

export { type Axis } from "../check/free.js";
export { Endpoint, Side } from "./endpoint.js";

export type Style = "corner" | "arc" | "bezier";

export type RouteError =
  /// Bad arguments; the CLI exits 3.
  | { kind: "Usage"; message: string }
  /// `<from>` or `<to>` doesn't resolve; the CLI exits 2.
  | { kind: "Diagnostics"; diags: Diagnostic[] };

export class Crossing {
  constructor(
    public line: string,
    public at: Pt,
  ) {}

  toJson(): Json {
    return { line: this.line, at: this.at.toJson() };
  }
}

/// A group the path passes through without connecting to anything inside it, and where it
/// enters, as `check` reports with `group-transit`.
export class Transit {
  constructor(
    public group: string,
    public at: Pt,
  ) {}

  toJson(): Json {
    return { group: this.group, at: this.at.toJson() };
  }
}

export class Corridor {
  constructor(
    /// `y` for a horizontal lane (the band is a y range), `x` for a vertical one.
    public axis: Axis,
    public band: [number, number],
  ) {}

  eq(o: Corridor): boolean {
    return this.axis === o.axis && this.band[0] === o.band[0] && this.band[1] === o.band[1];
  }

  toJson(): Json {
    return { axis: this.axis, band: [new Float(this.band[0]), new Float(this.band[1])] };
  }
}

export interface CandidateFields {
  crossings: number;
  crossed: Crossing[];
  transits: number;
  transited: Transit[];
  near: number;
  segments: number;
  length: number;
  corridor: Corridor | null;
  /// A ready-to-paste `line` statement, or `arrow` when rerouting one.
  statement: string;
  /// The rerouted line's `allow=` entries that suppress nothing on this path, left out of
  /// `statement` (with `reason=` when no entry is left).
  droppedAllow: string[];
}

export interface Candidate extends CandidateFields {}

export class Candidate {
  constructor(f: CandidateFields) {
    Object.assign(this, f);
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      crossings: this.crossings,
      crossed: this.crossed.map((c) => c.toJson()),
      transits: this.transits,
      transited: this.transited.map((t) => t.toJson()),
      near: this.near,
      segments: this.segments,
      length: new Float(this.length),
      corridor: this.corridor === null ? null : this.corridor.toJson(),
      statement: this.statement,
    };
    if (this.droppedAllow.length > 0) o["dropped_allow"] = [...this.droppedAllow];
    return o;
  }
}

export interface SuggestionFields {
  from: Endpoint;
  to: Endpoint;
  style: Style;
  candidates: Candidate[];
  /// Existing lines between the same two shapes, in either direction, when no `--line` was
  /// given; they count as obstacles for the candidates.
  existingLines: string[];
}

export interface Suggestion extends SuggestionFields {}

export class Suggestion {
  constructor(f: SuggestionFields) {
    Object.assign(this, f);
  }

  toJson(): Json {
    const o: { [key: string]: Json } = {
      from: this.from.toJson(),
      to: this.to.toJson(),
      style: this.style,
      candidates: this.candidates.map((c) => c.toJson()),
    };
    if (this.existingLines.length > 0) o["existing_lines"] = [...this.existingLines];
    return o;
  }
}

export interface Request {
  src: string;
  from: string | null;
  to: string | null;
  style: Style;
  /// `--line <id>`: reroute this existing line.
  line: string | null;
  radius: number;
  max: number;
  /// How the input's files resolve.
  opts: Options;
}

export function suggest(req: Request): Result<Suggestion, RouteError> {
  const { file } = parse(req.src);
  const [scene] = resolveWith(file, req.opts);
  const src = Source.new(req.src);
  let target: Target | null = null;
  if (req.line !== null) {
    const t = existing(file, scene, src, req.line);
    if (!t.ok) return t;
    target = t.value;
  }
  let from: Endpoint;
  let to: Endpoint;
  if (req.from !== null && req.to !== null) {
    const r = resolveEndpoints(req.src, req.from, req.to, req.opts);
    if (!r.ok) return r;
    [from, to] = r.value;
  } else if (req.from === null && req.to === null && target !== null) {
    [from, to] = [target.from, target.to];
  } else {
    return err({
      kind: "Usage",
      message: "give `<from> <to>`, or `--line <id>` to reroute a line",
    });
  }
  const arrow = target !== null && target.arrow;
  const [id, as, frame]: [string, Attrs, Frame] =
    target !== null
      ? [
          target.id,
          attrs(target.attrs, req.style, req.radius, scene.ortho),
          { origin: target.origin, after: target.end },
        ]
      : [
          defaultId(scene, from, to),
          attrs(null, req.style, req.radius, scene.ortho),
          Frame.default(),
        ];
  const existingLines = target !== null ? [] : connectingLines(file, scene, from, to);
  const candidates = rank(req, scene, src, [from, to], [arrow, id, as], frame);
  return ok(new Suggestion({ from, to, style: req.style, candidates, existingLines }));
}

/// Named lines that resolve and run between the owners of `from` and `to`, either way.
function connectingLines(file: ast.File, scene: Scene, from: Endpoint, to: Endpoint): string[] {
  const a = from.owner;
  const b = to.owner;
  if (a === null || b === null) {
    return [];
  }
  const owner = (p: ast.Point | null): string | null => {
    if (p === null) return null;
    const base = pointBase(p);
    return base.kind === "Ref" ? base.r.id.name : null;
  };
  const resolves = (id: string) => scene.lines.some((l) => l.id === id && l.path.length > 0);
  const out: string[] = [];
  for (const l of astLines(file.stmts)) {
    if (l.id === null) continue;
    const id = l.id.name;
    const x = owner(l.start);
    if (x === null) continue;
    const y = owner(endPoint(l));
    if (y === null) continue;
    if (((x === a && y === b) || (x === b && y === a)) && resolves(id)) {
      out.push(id);
    }
  }
  return out;
}

export function astLines(stmts: readonly ast.Stmt[]): ast.Line[] {
  const out: ast.Line[] = [];
  for (const s of stmts) {
    switch (s.kind) {
      case "Line":
        out.push(s.value);
        break;
      case "Group":
        out.push(...astLines(s.value.children));
        break;
      default:
        break;
    }
  }
  return out;
}

/// Where a candidate is written: the origin of its frame, and the source position right
/// after which the probes go, so their literals mean the same as in the final statement.
export interface Frame {
  origin: Pt;
  /// 1-based line and exclusive column; `null` appends the probes at the end of the file.
  after: [number, number] | null;
}

export const Frame = {
  default(): Frame {
    return { origin: Pt.default(), after: null };
  },
};

/// An existing line to reroute: its endpoints as written, its attributes' source text, the
/// origin of the frame its literals are written in and where its statement ends.
export interface Target {
  id: string;
  /// Written as an `arrow` statement.
  arrow: boolean;
  from: Endpoint;
  to: Endpoint;
  attrs: Written[];
  origin: Pt;
  end: [number, number];
}

export function existing(
  file: ast.File,
  scene: Scene,
  src: Source,
  id: string,
): Result<Target, RouteError> {
  const astLine = astLines(file.stmts).find((l) => l.id !== null && l.id.name === id);
  if (astLine === undefined) {
    return err({ kind: "Usage", message: `no line \`${id}\` in the file` });
  }
  const line = scene.lines.find((l) => l.id === id);
  if (line === undefined) {
    return err({
      kind: "Usage",
      message: `line \`${id}\` doesn't resolve; fix it with \`kashika check\` first`,
    });
  }
  const first = line.path[0];
  const last = line.path[line.path.length - 1];
  if (first === undefined || last === undefined) {
    return err({ kind: "Usage", message: `line \`${id}\` has no segments` });
  }
  const endpoint = (p: ast.Point | null, at: Pt) =>
    p !== null ? Endpoint.of(p, src.slice(p.span), at) : Endpoint.literal(at);
  return ok({
    id,
    arrow: astLine.arrow,
    from: endpoint(astLine.start, segFrom(first)),
    to: endpoint(endPoint(astLine), segTo(last)),
    attrs: astLine.attrs.map(
      (a): Written => ({
        key: a.key.name,
        text: src.slice(a.span),
        allow: a.value.kind === "Word" && a.key.name === "allow" ? a.value.value.split(",") : [],
      }),
    ),
    origin: line.origin,
    end: [astLine.span.endLine, astLine.span.endCol],
  });
}

export function probeId(i: number): string {
  return `${PROBE}_${i}`;
}

/// Scores every plan as a real line. All probes are resolved in one compile, then each is
/// checked on its own against the scene, in the place its statement would take.
function rank(
  req: Request,
  scene: Scene,
  src: Source,
  [from, to]: [Endpoint, Endpoint],
  [arrow, id, as]: [boolean, string, Attrs],
  frame: Frame,
): Candidate[] {
  const ctx = Ctx.new(scene);
  const skip: string[] = req.line === null ? [] : [req.line];
  const plans = generate(ctx, from, to, req.style, skip);
  const paths: string[] = plans.map((p) => pathText(from, to, p, frame.origin));
  const all = as.text(null);
  const probes: string[] = paths.map((p, i) => statement(arrow, probeId(i), p, all));
  const [text, first] = src.withLines(frame.after, probes);
  const [probed, diags] = compileWith(text, req.opts);
  const failed: number[] = diags
    .filter((d) => d.severity === "error")
    .filter((d) => d.span.line >= first)
    .map((d) => d.span.line - first);
  const at = probed.lines.findIndex((l) => l.id.startsWith(PROBE));
  if (at < 0) {
    return [];
  }
  const resolved = new Map<string, Line>();
  for (const l of probed.lines.slice(at)) {
    resolved.set(l.id, l);
  }
  const work = new Scene({ ...scene, lines: [...scene.lines] });
  const seen = new Set<string>();
  const candidates: Candidate[] = [];
  const detour: boolean[] = [];
  const ties: LaneAnchors[] = [];
  for (let i = 0; i < plans.length; i++) {
    const plan = plans[i]!;
    const path = paths[i]!;
    if (failed.includes(i)) {
      continue;
    }
    const line = resolved.get(probeId(i));
    if (line === undefined) {
      continue;
    }
    if (!directionsOk(line, from, to, req.style)) {
      continue;
    }
    const shape = key(flatten(line.path))
      .map(([x, y]) => `${x},${y}`)
      .join(";");
    if (seen.has(shape)) {
      continue;
    }
    seen.add(shape);
    work.lines.splice(at, 0, line);
    const s = score.score(work, work.lines[at]!, req.line);
    work.lines.splice(at, 1);
    if (s === null) {
      continue;
    }
    let corridor: Corridor | null = null;
    if (plan.lane !== null) {
      const [axis, v] = plan.lane;
      const span = laneSpan(from, to, plan, axis, v);
      if (span !== null) {
        corridor = score.corridor(ctx, axis, v, span, skip);
      }
    }
    const kept: string[] = [];
    const droppedAllow: string[] = [];
    for (const a of as.allow()) {
      if (s.findings.some((d) => score.suppresses(a, line.id, d))) {
        kept.push(a);
      } else {
        droppedAllow.push(a);
      }
    }
    detour.push(plan.hops.length >= 3);
    ties.push(laneAnchors(plan, from, to));
    candidates.push(
      new Candidate({
        crossings: s.crossed.length,
        crossed: s.crossed,
        transits: s.transited.length,
        transited: s.transited,
        near: s.near,
        segments: segments(line.path, req.style),
        length: round(s.length),
        corridor,
        statement: statement(arrow, id, path, as.text(kept)),
        droppedAllow,
      }),
    );
  }
  const simple: Candidate[] = candidates.filter((_, i) => !detour[i]);
  const drop: boolean[] = candidates.map((c, i) => detour[i]! && dominated(c, simple));
  const ranked: [LaneAnchors, Candidate][] = candidates
    .map((c, i): [LaneAnchors, Candidate] => [ties[i]!, c])
    .filter((_, i) => !drop[i]);
  ranked.sort(
    ([ta, a], [tb, b]) =>
      cmpTuple(
        [a.crossings + a.transits, a.near, a.segments],
        [b.crossings + b.transits, b.near, b.segments],
      ) ||
      totalCmp(a.length, b.length) ||
      cmpTuple(ta, tb),
  );
  return ranked.slice(0, req.max).map(([, c]) => c);
}

/// A detour is dropped when a simpler candidate is at least as good on crossings and
/// transits, near-misses and length.
function dominated(c: Candidate, simple: readonly Candidate[]): boolean {
  return simple.some(
    (s) =>
      s.segments < c.segments &&
      s.crossings + s.transits <= c.crossings + c.transits &&
      s.near <= c.near &&
      s.length <= c.length,
  );
}

/// The extent, on the other axis, of the segment that runs along a plan's lane.
function laneSpan(
  from: Endpoint,
  to: Endpoint,
  plan: Plan,
  axis: Axis,
  v: number,
): [number, number] | null {
  const pts = corners(from.at, to.at, plan);
  if (pts === null) return null;
  for (let i = 0; i + 1 < pts.length; i++) {
    const p = pts[i]!;
    const q = pts[i + 1]!;
    if (axis === "x" && p.x === v && q.x === v) {
      return [p.y, q.y];
    } else if (axis === "y" && p.y === v && q.y === v) {
      return [p.x, q.x];
    }
  }
  return null;
}

/// Orthogonal styles must stay orthogonal. A path must leave a side anchor along its outward
/// normal and enter a side anchor against it, so the arrowhead lands square on the edge.
function directionsOk(line: Line, from: Endpoint, to: Endpoint, style: Style): boolean {
  const first = line.path[0];
  const last = line.path[line.path.length - 1];
  if (first === undefined || last === undefined) {
    return false;
  }
  if (style !== "bezier" && line.path.some((s) => s.kind === "line" && !isAxisAligned(s))) {
    return false;
  }
  const dot = (a: Pt, b: Pt) => a.x * b.x + a.y * b.y;
  return (
    (from.side === null || dot(startDir(first), Side.outward(from.side)) > SQUARE) &&
    (to.side === null || dot(endDir(last), Side.outward(to.side)) < -SQUARE)
  );
}

/// Cosine of the largest angle, about 2.5°, between a path's end and a side's normal.
export const SQUARE = 0.999;

function segments(path: readonly PathSeg[], style: Style): number {
  switch (style) {
    case "arc":
      return 1 + path.filter((s) => s.kind === "quad").length;
    case "corner":
    case "bezier":
      return path.length;
  }
}

function defaultId(scene: Scene, from: Endpoint, to: Endpoint): string {
  const base = `${from.owner ?? "p"}_${to.owner ?? "p"}`;
  const taken = (id: string) =>
    scene.shapes.some((s) => s.id === id) ||
    scene.lines.some((l) => l.id === id) ||
    scene.junctions.some((j) => j.id === id) ||
    scene.texts.some((t) => t.id === id);
  if (!taken(base)) {
    return base;
  }
  for (let n = 2; ; n++) {
    const c = `${base}_${n}`;
    if (!taken(c)) return c;
  }
}

/// An attribute of the rerouted line as written, with its `allow=` entries.
export interface Written {
  key: string;
  text: string;
  allow: string[];
}

/// Attributes for the suggested statement: the rerouted line's own, minus `radius` (and
/// `ortho` for curves), or `end=arrow` for a new line; `arc` adds `radius=N`, and a curve
/// under the diagram's `ortho` adds `ortho=false`.
export class Attrs {
  constructor(readonly written: Written[]) {}

  allow(): string[] {
    return this.written.flatMap((w) => w.allow);
  }

  /// The attributes, with only the `allow=` entries in `kept` when given, and no `reason=`
  /// when none is left.
  text(kept: readonly string[] | null): string {
    const k = kept !== null && this.allow().length > 0 ? kept : null;
    const out: string[] = [];
    for (const w of this.written) {
      if (k !== null && k.length === 0 && (w.key === "allow" || w.key === "reason")) {
        continue;
      }
      if (k !== null && w.key === "allow" && w.allow.length > 0) {
        const list = w.allow.filter((a) => k.includes(a));
        if (list.length > 0) out.push(`allow=${list.join(",")}`);
        continue;
      }
      out.push(w.text);
    }
    return out.join(" ");
  }
}

export function attrs(
  existing: readonly Written[] | null,
  style: Style,
  radius: number,
  diagramOrtho: boolean,
): Attrs {
  const curve = style === "bezier";
  const written = (text: string): Written => ({ key: "", text, allow: [] });
  const out: Written[] =
    existing !== null
      ? existing
          .filter((a) => a.key !== "radius" && !(curve && a.key === "ortho"))
          .map((a) => ({ ...a, allow: [...a.allow] }))
      : [written("end=arrow")];
  if (style === "arc") {
    out.push(written(`radius=${fmtNum(radius)}`));
  }
  if (curve && diagramOrtho) {
    out.push(written("ortho=false"));
  }
  return new Attrs(out);
}
