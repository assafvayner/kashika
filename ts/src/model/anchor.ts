import { type Result, err, fmax, ok } from "../diag/rust.js";
import { Pt, type Rect } from "../geom/index.js";
import { suggestWord } from "./attrs.js";

/// The anchors of a shape, with corners by their short names.
export const ANCHORS: readonly string[] = [
  "top",
  "bottom",
  "left",
  "right",
  "center",
  "tl",
  "tr",
  "bl",
  "br",
];

/// Every anchor name, the long corner names included.
export const ANCHOR_NAMES: readonly string[] = [
  "top",
  "bottom",
  "left",
  "right",
  "center",
  "tl",
  "tr",
  "bl",
  "br",
  "top-left",
  "top-right",
  "bottom-left",
  "bottom-right",
];

function canonical(name: string): string {
  switch (name) {
    case "top-left":
      return "tl";
    case "top-right":
      return "tr";
    case "bottom-left":
      return "bl";
    case "bottom-right":
      return "br";
    default:
      return name;
  }
}

/// Resolves an anchor on a (possibly rounded) rectangle. Edge anchors are axis-projected
/// onto the rounded outline; corner anchors stay on the bounding box.
/// Errors carry a message and an optional help line; `owner` is the id the anchor is on.
export function resolve(
  rect: Rect,
  radius: number,
  owner: string,
  name: string,
  t: number | null,
): Result<Pt, [string, string | null]> {
  const nameC = canonical(name);
  const isSide = nameC === "top" || nameC === "bottom" || nameC === "left" || nameC === "right";
  if (t !== null && !isSide && ANCHOR_NAMES.includes(name)) {
    return err([
      "`@t` only applies to `top`/`bottom`/`left`/`right`",
      `use \`${owner}.${name}\` without \`@t\`, or an edge such as \`${owner}.top@0.25\``,
    ]);
  }
  const tt = t ?? 0.5;
  if (isSide && !(tt >= 0.0 && tt <= 1.0)) {
    return err([
      "`t` must be between 0 and 1",
      `\`${owner}.${name}@0\` is the start of the edge and \`${owner}.${name}@1\` the end`,
    ]);
  }
  const { x, y, w, h } = rect;
  let p: Pt;
  switch (nameC) {
    case "top":
      p = new Pt(x + w * tt, y + inset(radius, w * tt, w));
      break;
    case "bottom":
      p = new Pt(x + w * tt, y + h - inset(radius, w * tt, w));
      break;
    case "left":
      p = new Pt(x + inset(radius, h * tt, h), y + h * tt);
      break;
    case "right":
      p = new Pt(x + w - inset(radius, h * tt, h), y + h * tt);
      break;
    case "center":
      p = rect.center();
      break;
    case "tl":
      p = new Pt(x, y);
      break;
    case "tr":
      p = new Pt(x + w, y);
      break;
    case "bl":
      p = new Pt(x, y + h);
      break;
    case "br":
      p = new Pt(x + w, y + h);
      break;
    default: {
      const s = suggestWord(name, ANCHOR_NAMES);
      const help = s === null ? null : `did you mean \`${s}\`?`;
      return err([`unknown anchor \`${name}\` on \`${owner}\``, help]);
    }
  }
  return ok(p);
}

/// How far a rounded outline sits inside its bounding-box edge at `along` px from the edge start.
function inset(r: number, along: number, len: number): number {
  let d: number;
  if (along < r) {
    d = r - along;
  } else if (along > len - r) {
    d = along - (len - r);
  } else {
    return 0.0;
  }
  return r - Math.sqrt(fmax(r * r - d * d, 0.0));
}
