import { type Ctx } from "../ctx.js";
import { type Sink } from "../sink.js";
import { nearAxis } from "./axis.js";
import { blockReducible } from "./block.js";
import { outOfCanvas } from "./canvas.js";
import { labelCollision } from "./collision.js";
import { lowContrast } from "./contrast.js";
import { cross } from "./cross.js";
import { portCrowding } from "./crowding.js";
import { diagonalTail } from "./diagonal.js";
import { endpointOffEdge } from "./endpoint.js";
import { missingGlyph } from "./glyph.js";
import { offGrid } from "./grid.js";
import { imageDistort, svgDropped } from "./image.js";
import { jog } from "./jog.js";
import { near } from "./near.js";
import { overlap } from "./overlap.js";
import { sharedSegment } from "./shared.js";
import { spill } from "./spill.js";
import { textOutsideShape } from "./textarea.js";
import { through } from "./through.js";
import { groupTransit } from "./transit.js";

export { unusedAllow } from "./unused.js";

export type Lint = (ctx: Ctx, out: Sink) => void;

/// Every warning lint.
export const ALL: readonly Lint[] = [
  blockReducible,
  outOfCanvas,
  labelCollision,
  lowContrast,
  cross,
  portCrowding,
  diagonalTail,
  endpointOffEdge,
  missingGlyph,
  offGrid,
  imageDistort,
  jog,
  svgDropped,
  near,
  nearAxis,
  overlap,
  sharedSegment,
  spill,
  textOutsideShape,
  through,
  groupTransit,
];
