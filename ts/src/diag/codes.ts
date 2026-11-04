/// Lint codes that other passes read back.
export const code = {
  CROSS: "cross",
  SHARED_SEGMENT: "shared-segment",
  THROUGH: "through",
  NEAR: "near",
  GROUP_TRANSIT: "group-transit",
  UNUSED_ALLOW: "unused-allow",
} as const;

/// Fact keys that other passes read back.
export const fact = {
  /// `[x, y]` where two lines cross.
  AT: "at",
  /// `[x, y]` where a line enters a shape.
  ENTRY: "entry",
  /// `[x, y]` where a line leaves a shape.
  EXIT: "exit",
} as const;

/// Every error code, in the order of the spec's §7 Errors table.
export const ERROR_CODES: readonly string[] = [
  "parse",
  "unknown-id",
  "unknown-anchor",
  "duplicate-id",
  "duplicate-style",
  "placement-cycle",
  "label-overflow",
  "text-conflict",
  "not-ortho",
  "degenerate-curve",
  "zero-length",
  "attr",
  "unknown-style",
  "unknown-shape",
  "size",
  "range",
  "diagram",
  "version",
  "file",
];

export const LINT_CODES: readonly string[] = [
  code.CROSS,
  code.SHARED_SEGMENT,
  code.THROUGH,
  code.NEAR,
  "overlap",
  "spill",
  "diagonal-tail",
  "near-axis",
  "jog",
  "endpoint-off-edge",
  "port-crowding",
  code.GROUP_TRANSIT,
  "low-contrast",
  "label-collision",
  "out-of-canvas",
  "off-grid",
  "block-reducible",
  "text-outside-shape",
  "image-distort",
  "svg-dropped",
  "missing-glyph",
  code.UNUSED_ALLOW,
];
