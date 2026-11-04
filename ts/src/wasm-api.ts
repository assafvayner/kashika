/// Checks and renders Kashika source to SVG, like the `kashika-wasm` bindings: a source
/// string with default `Options`, for editors and Markdown previews. Built-in shapes and
/// bundled icons always work; unlike in WebAssembly, file references resolve from the
/// current directory under Node.

import { check } from "./check/index.js";
import { type Diagnostic, type Severity, sort } from "./diag/diagnostic.js";
import { Ordered, toJsonString } from "./diag/json.js";
import { Options, compileFile } from "./model/index.js";
import { TOOL_VERSION } from "./model/version.js";
import { renderSvgWith } from "./render/index.js";

export interface Rendered {
  /// The SVG, or `null` when there are errors and `force` is off, or the source is a
  /// library.
  svg: string | null;
  errors: number;
  warnings: number;
  /// Every diagnostic, suppressed ones included, in report order.
  diagnostics: Diagnostic[];
}

/// Compiles, checks and renders `src`. With `force`, a diagram with errors still renders,
/// with error markers like `kashika render --force`.
export function render(src: string, force: boolean): Rendered {
  const compiled = compileFile(src, new Options());
  const diags = compiled.diags;
  let svg: string | null = null;
  if (compiled.scene !== null) {
    const scene = compiled.scene;
    diags.push(...check(scene));
    sort(diags);
    const errors = count(diags, "error");
    if (errors === 0 || force) {
      svg = renderSvgWith(
        scene,
        diags.filter((d) => !d.suppressed),
        { errorMarkers: errors > 0, debug: false },
      );
    }
  }
  sort(diags);
  return {
    svg,
    errors: count(diags, "error"),
    warnings: count(diags, "warn"),
    diagnostics: diags,
  };
}

function count(diags: readonly Diagnostic[], severity: Severity): number {
  return diags.filter((d) => d.severity === severity && !d.suppressed).length;
}

/// `render` as a JSON string: `{svg, errors, warnings, diagnostics}`.
export function renderJson(src: string, force: boolean): string {
  const r = render(src, force);
  return toJsonString(
    new Ordered([
      ["svg", r.svg],
      ["errors", r.errors],
      ["warnings", r.warnings],
      ["diagnostics", r.diagnostics.map((d) => d.toJson())],
    ]),
  );
}

/// The Kashika version these bindings were built from.
export function version(): string {
  return TOOL_VERSION;
}
