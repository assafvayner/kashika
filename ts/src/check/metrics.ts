import { code } from "../diag/codes.js";
import { type Diagnostic } from "../diag/diagnostic.js";
import { round, sum } from "../diag/rust.js";
import { length } from "../geom/index.js";
import { type Scene } from "../model/index.js";

export interface Metrics {
  crossings: number;
  wirePx: number;
}

/// Summary numbers for the report: crossings (suppressed ones included) and total line length.
export function metrics(scene: Scene, diags: readonly Diagnostic[]): Metrics {
  const crossings = diags.filter((d) => d.code === code.CROSS).length;
  return metricsWith(scene, crossings);
}

export function metricsWith(scene: Scene, crossings: number): Metrics {
  const wire = sum(scene.lines.map((l) => length(l.path)));
  const px = round(wire);
  return { crossings, wirePx: Number.isNaN(px) || px < 0 ? 0 : px };
}
