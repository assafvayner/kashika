import { Diagnostic, Span, cmpSeverity } from "../diag/diagnostic.js";
import { didYouMean } from "../diag/distance.js";
import { type Result, asUsize, cmpStr, cmpTuple, displayF64, err, ok } from "../diag/rust.js";
import { Rect, parseUnitPath } from "../geom/index.js";
import {
  type Attr,
  type File,
  type Import,
  type Quad,
  type ShapeDef,
  type Stmt,
  type StyleDef,
  parse,
  stmtSpan,
} from "../syntax/index.js";
import { Store } from "./assets.js";
import * as attrs from "./attrs.js";
import { Attrs } from "./attrs.js";
import { bundled } from "./icons.js";
import { FileError, type Loader, pathParent } from "./loader.js";
import { Fit } from "./scene.js";
import { SHAPES, type UnitShape, builtinShape, shapeNames } from "./shapes.js";
import * as style from "./style.js";
import { type StyleProps } from "./style.js";

/// Deepest chain of nested imports followed before giving up.
const MAX_IMPORT_DEPTH = 32;
/// Most library instances (a file under one namespace) registered for one diagram.
const MAX_INSTANCES = 1000;
/// Largest library file read.
const MAX_IMPORT_BYTES = 1024 * 1024;
/// Keys a `shape <name> from "<file.svg>"` takes.
export const SHAPE_FROM_KEYS: readonly string[] = ["outline", "text-area", "fit"];
/// Import chains longer than this print only their first two and last hops.
const HOPS_SHOWN = 4;

export type LibShape = { kind: "unit"; value: UnitShape } | { kind: "svg"; value: SvgShape };

export const LibShape = {
  /// The unit text area before padding; `null` when the text sits below the shape.
  textArea(s: LibShape): Rect | null {
    if (s.kind === "unit") return s.value.textArea;
    if (s.value.below) return null;
    return new Rect(0.0, 0.0, 1.0, 1.0);
  },
};

/// A shape drawn from an SVG file.
export interface SvgShape {
  /// Index into the asset store.
  asset: number;
  /// `outline=hull`: the drawing's convex hull instead of the box.
  hull: boolean;
  /// `text-area=below`.
  below: boolean;
  /// How the drawing fills the box; `contain` unless `fit=` says otherwise.
  fit: Fit;
}

/// The library file a definition came from and the main file's import that brought it in.
export class Origin {
  constructor(
    /// The library's path as its importer wrote it.
    readonly file: string,
    /// The path string of the main file's `import` that leads to the library.
    readonly importSpan: Span,
  ) {}

  toString(): string {
    return `in \`${this.file}\` (imported at ${this.importSpan.line}:${this.importSpan.col})`;
  }
}

/// A library file, read, parsed and partly resolved once.
interface Module {
  path: string;
  dir: string;
  file: File;
  parseDiags: Diagnostic[];
  /// Per `style` statement index: its properties and the diagnostics they raised.
  styles: Map<number, [StyleProps, Diagnostic[]]>;
  /// Per `import` statement index: the file it names.
  imports: Map<number, Result<string, FileError>>;
}

export interface Ctx {
  loader: Loader;
  store: Store;
  diags: Diagnostic[];
  /// Diagnostics from imported files, not yet prefixed.
  nested: Nested[];
}

/// A copy of `d` that shares nothing mutable with it.
export function cloneDiag(d: Diagnostic): Diagnostic {
  const c = Object.assign(Object.create(Diagnostic.prototype) as Diagnostic, d);
  c.elements = [...d.elements];
  c.facts = new Map(d.facts);
  c.help = [...d.help];
  c.suggestions = [...d.suggestions];
  return c;
}

/// A diagnostic from inside an import, with the chain of imports that leads to it.
export class Nested {
  constructor(
    /// The import's path string in the file being processed.
    public at: Span,
    /// (import path, line, column) per file along the chain; the last is where `diag` is.
    public hops: [string, number, number][],
    public diag: Diagnostic,
  ) {}

  finish(): Diagnostic {
    const hop = ([path, line, col]: [string, number, number]) => `in \`${path}\` at ${line}:${col}`;
    const n = this.hops.length;
    const prefix: string[] =
      n > HOPS_SHOWN
        ? [
            hop(this.hops[0]!),
            hop(this.hops[1]!),
            `… ${n - 3} more imports …`,
            hop(this.hops[n - 1]!),
          ]
        : this.hops.map(hop);
    const d = cloneDiag(this.diag);
    d.span = this.at;
    d.message = `${prefix.join(": ")}: ${this.diag.message}`;
    d.elements = [];
    d.suggestions = [];
    return d;
  }
}

/// Where an import is in the walk: the files above it and how many of them are libraries.
interface Chain {
  stack: string[];
  depth: number;
  /// The path string of the main file's `import` being followed.
  top: Span;
}

export class Library {
  shapes: Map<string, LibShape> = new Map();
  /// Imported styles in load order, with qualified names. The main file's own styles
  /// are read by the resolver.
  styles: [string, StyleProps][] = [];
  /// (canonical path, namespace prefix) of every library instance registered so far.
  private loaded: Set<string> = new Set();
  /// Every library file read so far, parsed once whatever its namespaces.
  private modules: Map<string, Result<Module, FileError>> = new Map();
  /// Shape bodies of library files by (path, statement index), resolved on first use.
  private bodies: Map<string, [LibShape | null, Diagnostic[]]> = new Map();
  /// Library instances registered so far, against `MAX_INSTANCES`.
  private instances = 0;
  /// Where each imported style was defined, by qualified name.
  private styleOrigins: Map<string, Origin> = new Map();
  /// Where each imported shape was defined, by qualified name.
  private shapeOrigins: Map<string, Origin> = new Map();

  static default(): Library {
    return new Library();
  }

  static load(file: File, ctx: Ctx): Library {
    const lib = new Library();
    checkImportOrder(file, ctx.diags);
    lib.mainDefs(file, ctx);
    return lib;
  }

  /// Loads a library file checked on its own, as if it were imported without `as`.
  static loadStandalone(file: File, ctx: Ctx): Library {
    const lib = new Library();
    const pending: [number, StyleDef][] = [];
    for (const stmt of file.stmts) {
      if (stmt.kind === "Style") {
        const s = stmt.value;
        const props = styleProps(s, ctx.diags);
        const k = lib.addStyle(s, "", props, null, ctx);
        if (k !== null) {
          pending.push([k, s]);
        }
      }
    }
    lib.mainDefs(file, ctx);
    lib.styleShapes(pending, "", ctx.diags);
    return lib;
  }

  /// The main file's shapes and imports.
  private mainDefs(file: File, ctx: Ctx): void {
    const root = ctx.loader.root();
    const main = ctx.loader.main();
    const chain: Chain = {
      stack: main === null ? [] : [main],
      depth: 0,
      top: Span.new(1, 1, 1, 1),
    };
    for (const stmt of file.stmts) {
      if (stmt.kind === "Shape") {
        const s = stmt.value;
        if (this.nameFree(s, "", ctx)) {
          const d = shapeBody(s, root, ctx);
          if (d !== null) {
            this.shapes.set(s.name.name, d);
          }
        }
      } else if (stmt.kind === "Import") {
        const i = stmt.value;
        const path = findImport(ctx.loader, root, i);
        this.import(i, path, "", chain, ctx);
      }
    }
    finish(ctx);
  }

  private import(
    i: Import,
    pathR: Result<string, FileError>,
    prefix: string,
    chain: Chain,
    ctx: Ctx,
  ): void {
    const fail = (ctx: Ctx, e: FileError) => pushFileError(ctx.diags, i.path_span, e);
    if (!pathR.ok) return fail(ctx, pathR.error);
    const path = pathR.value;
    const pos = chain.stack.findIndex((p) => p === path);
    if (pos >= 0) {
      const names = [...chain.stack.slice(pos), path].map(fileName);
      return fail(ctx, new FileError(`import cycle: ${names.join(" -> ")}`));
    }
    if (chain.depth >= MAX_IMPORT_DEPTH) {
      return fail(ctx, new FileError(`imports are nested more than ${MAX_IMPORT_DEPTH} deep`));
    }
    const ns = i.ns !== null ? `${prefix}${i.ns.name}.` : prefix;
    const loadedKey = `${path}\u0000${ns}`;
    if (this.loaded.has(loadedKey)) {
      return;
    }
    if (this.instances >= MAX_INSTANCES) {
      if (this.instances === MAX_INSTANCES) {
        this.instances += 1;
        fail(
          ctx,
          new FileError(
            `more than ${MAX_INSTANCES} library imports in total, counting each namespace a library is imported under`,
            "import shared libraries once, instead of under several namespaces at each level",
          ),
        );
      }
      return;
    }
    this.instances += 1;
    this.loaded.add(loadedKey);
    if (chain.depth === 0) {
      chain.top = i.path_span;
    }
    const origin = new Origin(i.path, chain.top);
    const mr = this.module(path, i.path, ctx.loader);
    if (!mr.ok) return fail(ctx, mr.error);
    const m = mr.value;
    let inner = m.parseDiags.map(cloneDiag);
    const outer = ctx.diags;
    ctx.diags = [];
    const outerNested = ctx.nested;
    ctx.nested = [];
    const pending: [number, StyleDef][] = [];
    m.file.stmts.forEach((stmt, k) => {
      switch (stmt.kind) {
        case "Shape":
        case "Import":
          break;
        case "Style": {
          const [props, diags] = m.styles.get(k)!;
          ctx.diags.push(...diags.map(cloneDiag));
          const at = this.addStyle(stmt.value, ns, props.clone(), origin, ctx);
          if (at !== null) {
            pending.push([at, stmt.value]);
          }
          break;
        }
        default:
          ctx.diags.push(
            Diagnostic.error(
              "parse",
              stmtSpan(stmt),
              `\`${keyword(stmt)}\` is not allowed in a library; library files may contain only \`shape\`, \`style\` and \`import\``,
            ),
          );
      }
    });
    chain.stack.push(path);
    chain.depth += 1;
    this.moduleDefs(m, ns, origin, chain, ctx);
    chain.depth -= 1;
    chain.stack.pop();
    this.styleShapes(pending, ns, ctx.diags);
    inner = inner.concat(ctx.diags);
    ctx.diags = [];
    const found: Nested[] = inner
      .map((d) => new Nested(d.span, [], d))
      .concat(ctx.nested);
    ctx.nested = [];
    found.sort((a, b) => {
      const c = cmpTuple([a.at.line, a.at.col], [b.at.line, b.at.col]);
      if (c !== 0) return c;
      const s = cmpSeverity(a.diag.severity, b.diag.severity);
      return s !== 0 ? s : cmpStr(a.diag.code, b.diag.code);
    });
    ctx.diags = outer;
    ctx.nested = outerNested;
    for (const n of found) {
      n.hops.unshift([i.path, n.at.line, n.at.col]);
      n.at = i.path_span;
      ctx.nested.push(n);
    }
  }

  /// The parsed library at `path`, read on first use.
  private module(path: string, written: string, loader: Loader): Result<Module, FileError> {
    const m = this.modules.get(path);
    if (m !== undefined) {
      return m;
    }
    const r = readModule(path, written, loader);
    this.modules.set(path, r);
    return r;
  }

  /// A library's shapes and imports, registered under `ns`.
  private moduleDefs(m: Module, ns: string, origin: Origin, chain: Chain, ctx: Ctx): void {
    m.file.stmts.forEach((stmt, k) => {
      if (stmt.kind === "Shape") {
        const s = stmt.value;
        if (!this.nameFree(s, ns, ctx)) {
          return;
        }
        const key = `${m.path}\u0000${k}`;
        if (!this.bodies.has(key)) {
          const outer = ctx.diags;
          ctx.diags = [];
          const def = shapeBody(s, m.dir, ctx);
          const diags = ctx.diags;
          ctx.diags = outer;
          this.bodies.set(key, [def, diags]);
        }
        const [def, diags] = this.bodies.get(key)!;
        ctx.diags.push(...diags.map(cloneDiag));
        if (def !== null) {
          const name = `${ns}${s.name.name}`;
          this.shapeOrigins.set(name, origin);
          this.shapes.set(name, def);
        }
      } else if (stmt.kind === "Import") {
        const path = m.imports.get(k)!;
        this.import(stmt.value, path, ns, chain, ctx);
      }
    });
  }

  /// Registers `s` under `ns`; its index in `styles`, or `null` when the name is taken.
  private addStyle(
    s: StyleDef,
    ns: string,
    props: StyleProps,
    origin: Origin | null,
    ctx: Ctx,
  ): number | null {
    const name = `${ns}${s.name}`;
    if (this.styles.some(([n]) => n === name)) {
      ctx.diags.push(
        Diagnostic.error(
          "duplicate-style",
          s.span,
          twice("style", name, this.styleOrigins.get(name) ?? null),
        ),
      );
      return null;
    }
    if (origin !== null) {
      this.styleOrigins.set(name, origin);
    }
    this.styles.push([name, props]);
    return this.styles.length - 1;
  }

  /// Resolves the `shape=` of each registered library style, once every shape under
  /// `ns` is known.
  private styleShapes(pending: readonly [number, StyleDef][], ns: string, diags: Diagnostic[]): void {
    for (const [k, s] of pending) {
      const entry = this.styles[k]!;
      boundaryKeys(entry[0], s, entry[1], diags);
      const a = styleAttr(s, "shape");
      if (a === null) {
        continue;
      }
      const shape = styleShape(entry[0], a, diags) ? shapeName(a, ns, this.shapes, diags) : null;
      entry[1].shape = shape;
    }
  }

  /// Where the imported style `name` was defined.
  styleOrigin(name: string): Origin | null {
    return this.styleOrigins.get(name) ?? null;
  }

  /// Whether `s` may be defined under `prefix`: not a built-in name, not defined yet.
  private nameFree(s: ShapeDef, prefix: string, ctx: Ctx): boolean {
    if (builtinShape(s.name.name) !== null) {
      ctx.diags.push(
        Diagnostic.error(
          "duplicate-style",
          s.name.span,
          `\`${s.name.name}\` is a built-in shape`,
        ).withHelp("pick another name"),
      );
      return false;
    }
    const name = `${prefix}${s.name.name}`;
    if (this.shapes.has(name)) {
      ctx.diags.push(
        Diagnostic.error(
          "duplicate-style",
          s.name.span,
          twice("shape", name, this.shapeOrigins.get(name) ?? null),
        ),
      );
      return false;
    }
    return true;
  }
}

function fileName(p: string): string {
  const parts = p.split(/[/\\]/).filter((x) => x !== "");
  return parts[parts.length - 1] ?? "";
}

function extension(p: string): string | null {
  const name = fileName(p);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1) : null;
}

/// The message for a second definition of `name`, naming where an imported first one is.
export function twice(kind: string, name: string, first: Origin | null): string {
  if (first !== null) {
    return `${kind} \`${name}\` is already defined ${first.toString()}`;
  }
  return `${kind} \`${name}\` is defined twice`;
}

function readModule(path: string, written: string, loader: Loader): Result<Module, FileError> {
  const bytes = loader.read({ kind: "file", path }, written, MAX_IMPORT_BYTES);
  if (!bytes.ok) return bytes;
  let src: string;
  try {
    src = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes.value);
  } catch {
    return err(new FileError(`cannot read \`${written}\`: stream did not contain valid UTF-8`));
  }
  const { file, diags: parseDiags } = parse(src);
  const dir = pathParent(path) ?? path;
  const styles = new Map<number, [StyleProps, Diagnostic[]]>();
  const imports = new Map<number, Result<string, FileError>>();
  file.stmts.forEach((stmt, k) => {
    if (stmt.kind === "Style") {
      const diags: Diagnostic[] = [];
      const props = styleProps(stmt.value, diags);
      styles.set(k, [props, diags]);
    } else if (stmt.kind === "Import") {
      imports.set(k, findImport(loader, dir, stmt.value));
    }
  });
  return ok({ path, dir, file, parseDiags, styles, imports });
}

/// The `.kash` file an import names, found from `dir`.
function findImport(loader: Loader, dir: string, i: Import): Result<string, FileError> {
  const f = loader.find(dir, i.path);
  if (!f.ok) return f;
  if (f.value.kind === "bundled") {
    return err(new FileError(`\`${i.path}\` is a bundled icon, not a library`));
  }
  const path = f.value.path;
  if (extension(path) !== "kash") {
    return err(new FileError(`imports must be \`.kash\` files; \`${i.path}\` is not`));
  }
  return ok(path);
}

/// The keys of a `style` definition, besides `shape`, which needs the shapes in scope.
export function styleProps(s: StyleDef, diags: Diagnostic[]): StyleProps {
  const a = new Attrs(s.attrs, "style", style.defKeys(), diags);
  const props = attrs.styleProps(a, diags);
  const x = a.get("size");
  if (x !== null) {
    props.size = attrs.size(x, diags);
  }
  return props;
}

/// The `key=` of a `style` definition that `Attrs` keeps: the first, without `@`.
export function styleAttr(s: StyleDef, key: string): Attr | null {
  return s.attrs.find((a) => a.key.name === key && a.at_t === null) ?? null;
}

/// Drops `size` and `text-valign` from the style `name` when it is `boundary`, which every
/// group starts from: a size there would size every group, and a group's title sits at its
/// top or bottom edge (`title=`).
export function boundaryKeys(
  name: string,
  s: StyleDef,
  props: StyleProps,
  diags: Diagnostic[],
): void {
  if (name !== "boundary") {
    return;
  }
  if (props.size !== null) {
    const span = styleAttr(s, "size")?.span ?? s.span;
    diags.push(
      Diagnostic.error(
        "attr",
        span,
        "the `boundary` style can't set `size`: every group starts from it",
      ).withHelp("drop `size` here, and set it with another style or inline on the group"),
    );
    props.size = null;
  }
  if (props.textValign !== null) {
    const span = styleAttr(s, "text-valign")?.span ?? s.span;
    diags.push(
      Diagnostic.error(
        "attr",
        span,
        "the `boundary` style can't set `text-valign`: every group starts from it, and a group's title sits at its top or bottom edge",
      ).withHelp(
        "drop `text-valign` here, and set it with another style or inline on the box; `title=bottom` puts group titles at the bottom",
      ),
    );
    props.textValign = null;
  }
}

/// Whether the style `name` may take the `shape=` in `a`: every group draws with the
/// `boundary` style, and groups take no shape.
export function styleShape(name: string, a: Attr, diags: Diagnostic[]): boolean {
  if (name !== "boundary") {
    return true;
  }
  diags.push(
    Diagnostic.error(
      "attr",
      a.span,
      "the `boundary` style can't set `shape`: every group starts from it, and groups take no shape",
    ).withHelp("drop `shape` here, and set it with another style or inline on the box"),
  );
  return false;
}

/// The keys of `shapes` in order, like a `BTreeMap`'s.
export function sortedKeys<V>(m: ReadonlyMap<string, V>): string[] {
  return [...m.keys()].sort(cmpStr);
}

/// The qualified shape that `shape=` names when written under namespace `ns`, if it
/// exists: a built-in, else `ns` followed by the name.
export function shapeName(
  a: Attr,
  ns: string,
  shapes: ReadonlyMap<string, LibShape>,
  diags: Diagnostic[],
): string | null {
  if (a.value.kind !== "Word") {
    diags.push(attrs.typeErr(a, "a shape name"));
    return null;
  }
  const w = a.value.value;
  if (builtinShape(w) !== null) {
    return w;
  }
  const name = `${ns}${w}`;
  if (shapes.has(name)) {
    return name;
  }
  const keys = sortedKeys(shapes);
  const names = shapeNames().concat(
    keys.filter((k) => k.startsWith(ns)).map((k) => k.slice(ns.length)),
  );
  const d = Diagnostic.error("unknown-shape", a.span, `unknown shape \`${w}\``);
  const lower = w.replace(/[A-Z]/g, (c) => c.toLowerCase());
  const keyword = SHAPES.find((s) => s.name === lower || s.keywords.includes(lower))?.name ?? null;
  const namespaced =
    keys.find((k) => k.endsWith(w) && k.slice(0, k.length - w.length).endsWith(".")) ?? null;
  const s = keyword ?? namespaced ?? didYouMean(w, names);
  if (s !== null) {
    d.withHelp(`did you mean \`${s}\`?`);
  }
  if (bundled(w) !== null) {
    d.withHelp(
      `\`${w}\` is a bundled icon: put it on a box with \`icon="${w}"\`, or draw the box as it with \`shape ${w} from "${w}"\``,
    );
  }
  const list = shapeNames().map((n) => `\`${n}\``);
  d.withHelp(`built-in shapes: ${list.join(", ")}`);
  diags.push(d);
  return null;
}

function shapeBody(s: ShapeDef, dir: string, ctx: Ctx): LibShape | null {
  const b = s.body;
  if (b.kind === "Paths") {
    const u = unitShape(b.path, b.outline, b.text_area, ctx.diags);
    return u === null ? null : { kind: "unit", value: u };
  }
  const v = svgShape(b.file, b.file_span, b.attrs, dir, ctx);
  return v === null ? null : { kind: "svg", value: v };
}

function finish(ctx: Ctx): void {
  const nested = ctx.nested;
  ctx.nested = [];
  for (const n of nested) {
    ctx.diags.push(n.finish());
  }
}

function keyword(stmt: Stmt): string {
  switch (stmt.kind) {
    case "Diagram":
      return "diagram";
    case "Style":
      return "style";
    case "Box":
      return "box";
    case "Group":
      return "group";
    case "Junction":
      return "junction";
    case "Text":
      return "text";
    case "Line":
      return "line";
    case "Import":
      return "import";
    case "Shape":
      return "shape";
    case "Image":
      return "image";
  }
}

/// Whether `file` is a library: no `diagram`, and only `shape`, `style` and `import`.
export function isLibrary(file: File): boolean {
  return (
    !file.failed_diagram &&
    file.stmts.length > 0 &&
    file.stmts.every((s) => s.kind === "Shape" || s.kind === "Style" || s.kind === "Import")
  );
}

/// Reports imports that follow any statement other than `diagram` or another `import`.
function checkImportOrder(file: File, diags: Diagnostic[]): void {
  let otherSeen = false;
  for (const stmt of file.stmts) {
    if (stmt.kind === "Diagram") {
      continue;
    }
    if (stmt.kind === "Import") {
      if (otherSeen) {
        diags.push(
          Diagnostic.error(
            "parse",
            stmt.value.span,
            "`import` must come right after `diagram`",
          ).withHelp("move every import to the top of the file, below the `diagram` line"),
        );
      }
      continue;
    }
    otherSeen = true;
  }
}

function svgShape(
  file: string,
  span: Span,
  list: readonly Attr[],
  dir: string,
  ctx: Ctx,
): SvgShape | null {
  const a = new Attrs(list, "shape", SHAPE_FROM_KEYS, ctx.diags);
  const ox = a.get("outline");
  const hull = (ox === null ? null : attrs.word(ox, ["bbox", "hull"], ctx.diags)) === "hull";
  const tx = a.get("text-area");
  const below =
    (tx === null ? null : attrs.word(tx, ["below", "inside"], ctx.diags)) === "below";
  const fx = a.get("fit");
  const fw = fx === null ? null : attrs.word(fx, Fit.NAMES, ctx.diags);
  const fit = (fw === null ? null : Fit.fromName(fw)) ?? "contain";
  const r = ctx.store.loadSvg(ctx.loader, dir, file, "`shape ... from`");
  if (r.ok) {
    return { asset: r.value, hull, below, fit };
  }
  pushFileError(ctx.diags, span, r.error);
  return null;
}

export function pushFileError(diags: Diagnostic[], span: Span, e: FileError): void {
  const d = Diagnostic.error("file", span, e.message);
  if (e.help !== null) {
    d.withHelp(e.help);
  }
  diags.push(d);
}

function unitShape(
  path: [string, Span],
  outlineIn: [number, Span] | null,
  textAreaIn: [Quad, Span] | null,
  diags: Diagnostic[],
): UnitShape | null {
  const parsed = parseUnitPath(path[0]);
  if (!parsed.ok) {
    diags.push(Diagnostic.error("parse", path[1], parsed.error));
    return null;
  }
  const subpaths = parsed.value;
  let outline: number;
  if (outlineIn === null) {
    outline = 0;
  } else {
    const [n, span] = outlineIn;
    if (n % 1 !== 0 || n < 1.0 || asUsize(n) > subpaths.length) {
      diags.push(
        Diagnostic.error(
          "parse",
          span,
          `\`outline ${displayF64(n)}\` names no subpath; the path has ${subpaths.length}`,
        ),
      );
      return null;
    }
    outline = asUsize(n) - 1;
  }
  if (subpaths[outline]!.length === 0) {
    diags.push(Diagnostic.error("parse", path[1], "the outline subpath has no segments"));
    return null;
  }
  let textArea: Rect;
  if (textAreaIn === null) {
    textArea = new Rect(0.0, 0.0, 1.0, 1.0);
  } else {
    const [[x, y, w, h], span] = textAreaIn;
    const inside =
      w > 0.0 && h > 0.0 && x >= 0.0 && y >= 0.0 && x + w <= 1.0 + 1e-9 && y + h <= 1.0 + 1e-9;
    if (!inside) {
      diags.push(
        Diagnostic.error(
          "parse",
          span,
          "`text-area` must be a unit rect (x,y,w,h) inside 0..1 with a positive size",
        ),
      );
      return null;
    }
    textArea = new Rect(x, y, w, h);
  }
  return { subpaths, outline, hiddenOutline: false, textArea };
}
