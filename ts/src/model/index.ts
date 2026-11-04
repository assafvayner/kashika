import { Diagnostic, sort } from "../diag/diagnostic.js";
import { type File, parse } from "../syntax/index.js";
import { Options } from "./loader.js";
import { resolveLibraryWith, resolveWith } from "./resolve.js";
import { type Scene } from "./scene.js";

export { ANCHORS } from "./anchor.js";
export { MAX_RASTER_PIXELS, MAX_RASTER_SIDE, decodeBundledIcon, probe } from "./assets.js";
export { Form } from "./form.js";
export { BUNDLED_ICONS, ICON_KEYWORDS } from "./icons.js";
export { type Font, PlacedLine, Run } from "../text/index.js";
export { Options } from "./loader.js";
export { attributeKeys, resolve, resolveLibraryWith, resolveWith } from "./resolve.js";
export * from "./scene.js";
export {
  type BuiltinShape,
  SHAPES,
  type ShapeSource,
  builtinShape,
  builtinSource,
  shapeNames,
} from "./shapes.js";
export { BuiltinStyle, type FontWeight, Padding, STYLES, StrokeStyle, Style } from "./style.js";
export { TextItem, type TextRole } from "./textlayout.js";

/// Parses and resolves a source file. Always returns a (possibly partial) scene, so tools
/// like `inspect` work on files with errors. Relative paths resolve from the current directory.
export function compile(src: string): [Scene, Diagnostic[]] {
  return compileWith(src, new Options());
}

/// Like `compile`, with the input's path, sandbox and search path from `opts`.
export function compileWith(src: string, opts: Options): [Scene, Diagnostic[]] {
  const { file, diags } = parse(src);
  const [scene, more] = resolveWith(file, opts);
  diags.push(...more);
  sort(diags);
  return [scene, diags];
}

/// A source file, parsed once and resolved.
export interface Compiled {
  file: File;
  /// The scene, or `null` for a library file (§3a), which resolves only its definitions.
  scene: Scene | null;
  /// Parse and resolve diagnostics, sorted.
  diags: Diagnostic[];
}

/// Parses `src` and resolves it as a library when it is one, otherwise as a diagram, like
/// `compileLibraryWith` and then `compileWith`.
export function compileFile(src: string, opts: Options): Compiled {
  const { file, diags } = parse(src);
  let scene: Scene | null;
  const lib = resolveLibraryWith(file, opts);
  if (lib !== null) {
    diags.push(...lib);
    scene = null;
  } else {
    const [s, more] = resolveWith(file, opts);
    diags.push(...more);
    scene = s;
  }
  sort(diags);
  return { file, scene, diags };
}

/// Checks a library file (§3a: no `diagram`, only `shape`, `style` and `import`) on its own,
/// resolving its definitions. `null` when `src` is not a library; compile it as a diagram.
export function compileLibraryWith(src: string, opts: Options): Diagnostic[] | null {
  const { file, diags } = parse(src);
  const more = resolveLibraryWith(file, opts);
  if (more === null) return null;
  diags.push(...more);
  sort(diags);
  return diags;
}
