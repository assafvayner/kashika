import { type Diagnostic } from "../diag/diagnostic.js";
import { type Scene } from "../model/index.js";
import { Ctx } from "./ctx.js";
import { resolve } from "./free.js";
import { ALL, unusedAllow } from "./lints/mod.js";
import { type Bounded, Sink } from "./sink.js";

export * as ctx from "./ctx.js";
export * as element from "./element.js";
export * as free from "./free.js";
export * as outline from "./outline.js";
export * as ports from "./ports.js";
export * as textBox from "./text_box.js";
export { Ctx, LineGeom, type Mark, type ShapeGeom } from "./ctx.js";
export { type Deferred, DeferredKind, Finding } from "./free.js";
export { type Metrics, metrics } from "./metrics.js";
export { Bounded, type Omitted } from "./sink.js";
export { Grid } from "./spatial.js";
export { suppress, suppresses } from "./suppress.js";

/// Runs every warning lint on the scene, `unused-allow` last, and marks the suppressed
/// findings.
export function check(scene: Scene): Diagnostic[] {
  const ctx = Ctx.new(scene);
  const found = lint(ctx, new Sink(scene, Infinity, null));
  return found.diags.map((d, i) => {
    resolve(ctx, d, found.deferred[i]!);
    return d;
  });
}

/// Like `check`, but without the costly help that `checkBounded` defers.
export function checkDeferred(scene: Scene): Diagnostic[] {
  return lint(Ctx.new(scene), new Sink(scene, Infinity, null)).diags;
}

/// Like `check`, with memory bounded for scenes with huge numbers of findings and the costly
/// help left in `Bounded.deferred`, for `free.resolve` to work out for the diagnostics that
/// are shown. Only findings that pass `filter` are reported. Of those with the same code and
/// suppression, the first `keep` in `sort` order are kept and the rest only counted in
/// `omitted`. Kept findings come in the order `checkDeferred` reports them.
export function checkBounded(
  ctx: Ctx,
  keep: number,
  filter: (d: Diagnostic) => boolean,
): Bounded {
  return lint(ctx, new Sink(ctx.scene, keep, filter));
}

function lint(ctx: Ctx, sink: Sink): Bounded {
  for (const l of ALL) {
    l(ctx, sink);
  }
  unusedAllow(sink);
  return sink.finish();
}
