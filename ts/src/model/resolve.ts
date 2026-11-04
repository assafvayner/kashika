import { Diagnostic, Span } from "../diag/diagnostic.js";
import { type Pt, type Rect } from "../geom/index.js";
import * as ast from "../syntax/ast.js";
import { type File, type Stmt, stmtSpan } from "../syntax/ast.js";
import { Store } from "./assets.js";
import * as attrs from "./attrs.js";
import { Attrs } from "./attrs.js";
import * as library from "./library.js";
import { type Ctx, Library } from "./library.js";
import { type FileError, Loader, Options } from "./loader.js";
import * as markup from "./markup.js";
import {
  type Allow,
  type Fit,
  type Label,
  type LabelSide,
  type Layer,
  type Literal,
  type Marker,
  type Outline,
  type PlacementEdge,
  type Align,
  Scene,
} from "./scene.js";
import * as style from "./style.js";
import { type Style, type StyleProps } from "./style.js";
import * as textlayout from "./textlayout.js";
import { type ShapeTextProps } from "./textlayout.js";
import * as version from "./version.js";
import { type Built, build } from "./resolve/build.js";
import { fitCanvas } from "./resolve/canvas.js";
import { allowList, sizeOk } from "./resolve/checks.js";
import { placementGraph } from "./resolve/placement.js";
import { shapeAttr, validate } from "./resolve/props.js";

export const NODE_KEYS: readonly string[] = [
  "shape",
  "style",
  "pin",
  "allow",
  "reason",
  "icon",
  "icon-pos",
  "icon-size",
  "layer",
];
export const IMAGE_KEYS: readonly string[] = ["pin", "fit", "layer", "allow", "reason"];
export const LINE_KEYS: readonly string[] = [
  "start",
  "end",
  "stroke",
  "stroke-width",
  "stroke-style",
  "style",
  "radius",
  "label",
  "label-side",
  "ortho",
  "allow",
  "reason",
];
export const TEXT_KEYS: readonly string[] = [
  "font-size",
  "text-align",
  "color",
  "muted",
  "size",
  "align",
];
const DIAGRAM_KEYS: readonly string[] = [
  "kashika",
  "grid",
  "clearance",
  "background",
  "icons",
  "allow",
  "margin",
  "ortho",
];
/// Room a fitted canvas leaves to the right of and below the drawing, unless `margin=` sets it.
const DEFAULT_MARGIN = 20.0;

export const JUNCTION_KEYS: readonly string[] = ["dot"];

/// The keys a `box` (or, with `group`, a `group`) takes.
export function nodeKeys(group: boolean): string[] {
  return NODE_KEYS.filter((k) => !group || !(k === "shape" || k === "layer"))
    .concat(style.STYLE_KEYS, textlayout.SHAPE_TEXT_KEYS)
    .filter((k) => group || k !== "title");
}

/// Every attribute key, by the statement that takes it: `diagram`, `style`, `box`, `group`,
/// `junction`, `text`, `line`, `image`, `shape from` and a text block's `text`.
export function attributeKeys(): [string, string[]][] {
  return [
    ["diagram", [...DIAGRAM_KEYS]],
    ["style", style.defKeys()],
    ["box", nodeKeys(false)],
    ["group", nodeKeys(true)],
    ["junction", [...JUNCTION_KEYS]],
    ["text", [...TEXT_KEYS]],
    ["line", [...LINE_KEYS]],
    ["image", [...IMAGE_KEYS]],
    ["shape from", [...library.SHAPE_FROM_KEYS]],
    ["block text", [...textlayout.BLOCK_ITEM_KEYS]],
  ];
}

/// Longest chain of placement references, counting group frames.
export const MAX_PLACEMENT_DEPTH = 256;
/// Cycles longer than this print only their first few members.
export const CYCLE_SHOWN = 6;

export type Item =
  | { kind: "Box"; value: ast.Node }
  | { kind: "Group"; value: ast.Group }
  | { kind: "Image"; value: ast.Image }
  | { kind: "Junction"; value: ast.Junction }
  | { kind: "Text"; value: ast.Text }
  | { kind: "Line"; value: ast.Line };

export const Item = {
  id(i: Item): ast.Ident | null {
    switch (i.kind) {
      case "Box":
        return i.value.id;
      case "Group":
        return i.value.node.id;
      case "Image":
        return i.value.id;
      case "Junction":
        return i.value.id;
      case "Text":
        return i.value.id;
      case "Line":
        return i.value.id;
    }
  },

  span(i: Item): Span {
    switch (i.kind) {
      case "Group":
        return i.value.node.span;
      default:
        return i.value.span;
    }
  },

  atSpan(i: Item): Span {
    switch (i.kind) {
      case "Box":
        return ast.pointSpan(i.value.at);
      case "Group":
        return ast.pointSpan(i.value.node.at);
      case "Image":
        return ast.pointSpan(i.value.at);
      case "Junction":
        return ast.pointSpan(i.value.at);
      case "Text":
        return ast.pointSpan(i.value.at);
      case "Line":
        return i.value.span;
    }
  },
};

export interface Entry {
  item: Item;
  parent: number | null;
  depth: number;
  /// What `validate` read from its attributes; `null` before that and for skipped entries.
  props: Props | null;
}

/// An entry's attributes, read by `validate`.
export type Props =
  /// A box, group or image.
  | { kind: "Node"; value: NodeProps }
  | { kind: "Junction"; dot: boolean }
  | { kind: "Text"; value: TextProps }
  | { kind: "Line"; value: LineProps };

export const Entry = {
  node(e: Entry): NodeProps | null {
    return e.props !== null && e.props.kind === "Node" ? e.props.value : null;
  },

  text(e: Entry): TextProps | null {
    return e.props !== null && e.props.kind === "Text" ? e.props.value : null;
  },

  dot(e: Entry): boolean {
    return e.props !== null && e.props.kind === "Junction" && e.props.dot;
  },

  /// Takes a line's props out; they are used once.
  takeLine(e: Entry): LineProps | null {
    if (e.props !== null && e.props.kind === "Line") {
      const p = e.props.value;
      e.props = null;
      return p;
    }
    return null;
  },
};

export interface NodeProps {
  outline: Outline;
  /// The inline `shape=`, else the `style=` style's; `null` for groups and boxes without one.
  shape: string | null;
  style: Style;
  pin: [string, number | null];
  /// The resolved size; `null` when it is invalid or can't be fitted. A group's fitted
  /// axes hold 0 until it is placed.
  size: [number, number] | null;
  /// The axes of a group's size fitted to its children.
  fitted: [boolean, boolean];
  text: ShapeTextProps;
  allow: Allow[];
  reason: string | null;
  icon: IconProps | null;
  image: ImageProps | null;
  layer: Layer;
}

export type IconPos = "Left" | "Top";

export interface ImageProps {
  asset: number | null;
  fit: Fit;
}

export interface IconProps {
  asset: number;
  pos: IconPos;
  size: number;
}

export interface LineProps {
  start: Marker;
  end: Marker;
  style: Style;
  labels: Label[];
  labelSide: LabelSide;
  labelSideSpan: Span;
  ortho: boolean;
  /// `ortho` comes from the diagram rather than the line.
  orthoInherited: boolean;
  allow: Allow[];
  reason: string | null;
  radius: number;
}

export interface TextProps {
  content: string;
  size: number;
  color: string;
  align: Align;
}

export function resolve(file: File): [Scene, Diagnostic[]] {
  return resolveWith(file, new Options());
}

export function resolveWith(file: File, opts: Options): [Scene, Diagnostic[]] {
  const r = new Resolver(Loader.new(opts), new Set(file.failed_ids.map((i) => i.name)));
  r.iconsDir(file);
  const ctx: Ctx = { loader: r.loader, store: r.store, diags: [], nested: [] };
  r.lib = Library.load(file, ctx);
  r.diags.push(...ctx.diags);
  r.styles(file);
  r.collect(file.stmts, null, 0);
  r.placed = r.entries.map(() => undefined);
  const scene = r.diagram(file);
  r.grid = scene.grid;
  validate(r);
  placementGraph(r);
  build(r, scene);
  fitCanvas(r, scene);
  scene.assets = r.store.intoVec();
  r.store = Store.default();
  return [scene, r.diags];
}

/// Resolves a library file checked on its own: its imports, shape files and style keys.
/// `null` when `file` is not a library (§3a), so it should be resolved as a diagram.
export function resolveLibraryWith(file: File, opts: Options): Diagnostic[] | null {
  if (!library.isLibrary(file)) {
    return null;
  }
  const loader = Loader.new(opts);
  const store = Store.default();
  const ctx: Ctx = { loader, store, diags: [], nested: [] };
  Library.loadStandalone(file, ctx);
  return ctx.diags;
}

export class Resolver {
  store: Store = Store.default();
  lib: Library = Library.default();
  entries: Entry[] = [];
  ids: Map<string, number> = new Map();
  userStyles: Map<string, StyleProps> = new Map();
  /// Per entry: `undefined` until placed, then its canvas rect (junctions use a zero-size
  /// rect), or `null` when it can't be placed.
  placed: (Rect | null | undefined)[] = [];
  /// The frame origin of each fitted group, known before its size; `null` when it can't
  /// be placed.
  frames: Map<number, Pt | null> = new Map();
  /// Scene items built so far, by entry; `null` once `build` has taken one or it failed.
  built: Map<number, Built | null> = new Map();
  /// Entries being placed; `placementGraph` keeps this chain within the depth limit.
  onStack: Set<number> = new Set();
  skipped: Set<number> = new Set();
  edges: PlacementEdge[] = [];
  literals: Literal[] = [];
  /// The diagram's `grid=`, which fitted sizes round up to.
  grid: number | null = null;
  /// Which canvas axes are `auto`, fitted to what is drawn.
  canvasFit: [boolean, boolean] = [false, false];
  /// The diagram's `margin=`, added after the drawing on a fitted axis.
  margin = DEFAULT_MARGIN;
  /// The diagram's `ortho`: every line is `ortho` unless it says `ortho=false`.
  ortho = false;
  diags: Diagnostic[] = [];

  constructor(
    readonly loader: Loader,
    /// Ids declared inside a skipped duplicate group or by a statement that failed to parse;
    /// references to them stay silent.
    readonly skippedIds: Set<string>,
  ) {}

  /// Adds the first `diagram` statement's `icons=` directory to the search path.
  iconsDir(file: File): void {
    const d = file.stmts.find((s) => s.kind === "Diagram");
    if (d === undefined || d.kind !== "Diagram") {
      return;
    }
    const x = d.value.attrs.find((a) => a.key.name === "icons");
    if (x === undefined) {
      return;
    }
    if (x.value.kind !== "Str") {
      this.diags.push(attrs.typeErr(x, "a directory path string"));
      return;
    }
    const r = this.loader.addIconsDir(x.value.value);
    if (!r.ok) {
      this.fileError(x.span, r.error);
    }
  }

  fileError(span: Span, e: FileError): void {
    library.pushFileError(this.diags, span, e);
  }

  diagram(file: File): Scene {
    const scene = new Scene({
      title: "",
      size: [0.0, 0.0],
      grid: null,
      clearance: 6.0,
      background: null,
      allow: [],
      shapes: [],
      junctions: [],
      texts: [],
      lines: [],
      assets: [],
      placement: [],
      literals: [],
      fitted: [false, false],
      ortho: false,
    });
    let seen = false;
    for (const stmt of file.stmts) {
      if (stmt.kind !== "Diagram") continue;
      const d = stmt.value;
      if (seen) {
        this.diags.push(
          Diagnostic.error("diagram", d.span, "only one `diagram` statement is allowed"),
        );
        continue;
      }
      seen = true;
      const first = file.first_stmt;
      if (first === null || first.line !== d.span.line || first.col !== d.span.col) {
        this.diags.push(
          Diagnostic.error("diagram", d.span, "`diagram` must be the first statement"),
        );
      }
      scene.title = markup.unescape(d.title);
      const { w, h } = d.size ?? { w: null, h: null };
      if (sizeOk(this, [w ?? 1.0, h ?? 1.0], d.span, "the diagram")) {
        scene.size = [w ?? 0.0, h ?? 0.0];
        this.canvasFit = [w === null, h === null];
      }

      const a = new Attrs(d.attrs, "diagram", DIAGRAM_KEYS, this.diags);
      let x: ast.Attr | null;
      if ((x = a.get("kashika")) !== null) {
        let shown: version.Problem | null;
        if (x.value.kind === "Str") {
          const c = version.check(x.value.value, version.TOOL_VERSION);
          shown = c.ok ? null : c.error;
        } else {
          shown = {
            message: "`kashika=` must be a quoted version string",
            help: version.formHelp(),
          };
        }
        if (shown !== null) {
          this.diags.push(Diagnostic.error("version", x.span, shown.message).withHelp(shown.help));
        }
      }
      if ((x = a.get("grid")) !== null) {
        scene.grid = attrs.numIn(x, "Positive", this.diags);
      }
      if ((x = a.get("clearance")) !== null) {
        const v = attrs.numIn(x, "NonNegative", this.diags);
        if (v !== null) {
          scene.clearance = v;
        }
      }
      if ((x = a.get("background")) !== null) {
        const c = attrs.color(x, this.diags);
        scene.background = c !== null && c !== "none" ? c : null;
      }
      if ((x = a.get("allow")) !== null) {
        scene.allow = allowList(this, x, false);
      }
      if ((x = a.get("ortho")) !== null) {
        this.ortho = attrs.flag(x, this.diags);
        scene.ortho = this.ortho;
      }
      if ((x = a.get("margin")) !== null) {
        if (w !== null && h !== null) {
          this.diags.push(
            Diagnostic.error(
              "attr",
              x.span,
              "`margin` applies only to a canvas that fits its content",
            ).withHelp(
              "remove `margin`, or leave out `size` (or write `size (w, auto)`) to fit the canvas",
            ),
          );
        } else {
          const v = attrs.numIn(x, "NonNegative", this.diags);
          if (v !== null) {
            this.margin = v;
          }
        }
      }
    }
    if (!seen && !file.failed_diagram) {
      const span = file.stmts.length > 0 ? stmtSpan(file.stmts[0]!) : Span.new(1, 1, 1, 1);
      this.diags.push(
        Diagnostic.error("diagram", span, "missing `diagram` statement").withHelp(
          'start the file with `diagram "<title>" size (w,h)`',
        ),
      );
    }
    return scene;
  }

  styles(file: File): void {
    for (const [name, props] of this.lib.styles) {
      this.userStyles.set(name, props.clone());
    }
    const local = new Set<string>();
    for (const stmt of file.stmts) {
      if (stmt.kind !== "Style") continue;
      const s = stmt.value;
      const props = library.styleProps(s, this.diags);
      library.boundaryKeys(s.name, s, props, this.diags);
      const x = library.styleAttr(s, "shape");
      if (x !== null) {
        if (library.styleShape(s.name, x, this.diags)) {
          props.shape = shapeAttr(this, x);
        }
      }
      const had = this.userStyles.has(s.name);
      this.userStyles.set(s.name, props);
      if (had) {
        const first = !local.has(s.name) ? this.lib.styleOrigin(s.name) : null;
        this.diags.push(
          Diagnostic.error("duplicate-style", s.span, library.twice("style", s.name, first)),
        );
      }
      local.add(s.name);
    }
  }

  collect(stmts: readonly Stmt[], parent: number | null, depth: number): void {
    for (const stmt of stmts) {
      let item: Item;
      switch (stmt.kind) {
        case "Box":
          item = { kind: "Box", value: stmt.value };
          break;
        case "Group":
          item = { kind: "Group", value: stmt.value };
          break;
        case "Junction":
          item = { kind: "Junction", value: stmt.value };
          break;
        case "Text":
          item = { kind: "Text", value: stmt.value };
          break;
        case "Line":
          item = { kind: "Line", value: stmt.value };
          break;
        case "Diagram":
          if (parent !== null) {
            this.diags.push(
              Diagnostic.error(
                "diagram",
                stmtSpan(stmt),
                "`diagram` is only allowed as the first top-level statement",
              ),
            );
          }
          continue;
        case "Image":
          if (parent !== null) {
            this.diags.push(
              Diagnostic.error("parse", stmtSpan(stmt), "`image` is only allowed at the top level"),
            );
            continue;
          }
          item = { kind: "Image", value: stmt.value };
          break;
        case "Import":
          this.topLevelOnly("import", stmtSpan(stmt), parent);
          continue;
        case "Shape":
          this.topLevelOnly("shape", stmtSpan(stmt), parent);
          continue;
        case "Style":
          this.topLevelOnly("style", stmtSpan(stmt), parent);
          continue;
      }
      const idx = this.entries.length;
      this.entries.push({ item, parent, depth, props: null });
      const id = Item.id(item);
      if (parent !== null && this.skipped.has(parent)) {
        this.skipped.add(idx);
        if (id !== null) {
          this.skippedIds.add(id.name);
        }
      } else if (id !== null) {
        this.register(id, idx);
      }
      if (stmt.kind === "Group") {
        this.collect(stmt.value.children, idx, depth + 1);
      }
    }
  }

  topLevelOnly(kw: string, span: Span, parent: number | null): void {
    if (parent !== null) {
      this.diags.push(Diagnostic.error("parse", span, `\`${kw}\` is only allowed at the top level`));
    }
  }

  register(id: ast.Ident, idx: number): void {
    const prev = this.ids.get(id.name);
    if (prev !== undefined) {
      this.skipped.add(idx);

      const line = Item.span(this.entries[prev]!.item).line;
      this.diags.push(
        Diagnostic.error(
          "duplicate-id",
          id.span,
          `\`${id.name}\` is already defined on line ${line}`,
        ).withHelp("ids are unique across the whole file"),
      );
    } else {
      this.ids.set(id.name, idx);
    }
  }
}
