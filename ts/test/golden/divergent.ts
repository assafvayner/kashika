/// Golden cases whose expected output the TypeScript build cannot reproduce byte for byte,
/// keyed by case name (`render/assets/image-svg`), with the reason.
const USER_SVG = "embeds a user SVG file, which usvg re-serializes differently";

export const DIVERGENT: ReadonlyMap<string, string> = new Map<string, string>([
  ["render/assets/icon-color", USER_SVG],
  ["render/assets/icons", USER_SVG],
  ["render/assets/images", USER_SVG],
  ["render/assets/svg-sanitized", USER_SVG],
  ["render/assets/svg-shape-fit", USER_SVG],
  ["render/assets/svg-shapes", USER_SVG],
  ["render/assets/svg-text", USER_SVG],
  ["render/raster/png-embedded-images", USER_SVG],
  ["render/text/shape-text-areas", USER_SVG],
]);
