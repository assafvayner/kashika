import { extname } from "node:path";

import { type Result, ok } from "../diag/rust.js";
import { svgToJpeg, svgToPng, svgToWebp } from "../render/index.js";

export type Kind = "svg" | "png" | "jpeg" | "webp";

export const SUPPORTED = ".svg, .png, .jpg, .jpeg or .webp";

export function kind(path: string): Kind | null {
  switch (extname(path).slice(1).toLowerCase()) {
    case "svg":
      return "svg";
    case "png":
      return "png";
    case "jpg":
    case "jpeg":
      return "jpeg";
    case "webp":
      return "webp";
    default:
      return null;
  }
}

export async function encode(kind: Kind, svg: string): Promise<Result<Uint8Array, string>> {
  switch (kind) {
    case "svg":
      return ok(new TextEncoder().encode(svg));
    case "png":
      return svgToPng(svg);
    case "jpeg":
      return svgToJpeg(svg);
    case "webp":
      return svgToWebp(svg);
  }
}
