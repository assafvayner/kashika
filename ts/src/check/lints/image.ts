import { Diagnostic } from "../../diag/diagnostic.js";
import { Float, type Json } from "../../diag/json.js";
import { cmpStr, displayF64, round } from "../../diag/rust.js";
import { fmtNum, round2 } from "../../geom/index.js";
import { type ImageRef, ShapeKind } from "../../model/index.js";
import { type Ctx } from "../ctx.js";
import { shapeEl } from "../element.js";
import { type Sink } from "../sink.js";

const MAX_DISTORTION = 0.05;

/// `image-distort`: `fit=stretch` changes the aspect ratio of an image or SVG-file shape by
/// more than 5%.
export function imageDistort(ctx: Ctx, out: Sink): void {
  const assets = ctx.scene.assets;
  for (const s of ctx.scene.shapes.filter((s) => s.kind !== "group")) {
    const d = s.imageDistortion(assets);
    const img = s.image;
    if (d === null || img === null) {
      continue;
    }
    if (d <= MAX_DISTORTION) {
      continue;
    }
    const [nw, nh] = assets[img.asset]!.natural;
    const r = img.rect;
    const on = s.kind === "image" ? "" : ` on shape \`${s.shape}\``;
    out.push(
      Diagnostic.warn(
        "image-distort",
        s.span,
        `${ShapeKind.name(s.kind)} '${s.id}' is stretched ${displayF64(round(d * 100.0))}% away from its ${fmtNum(nw)}x${fmtNum(nh)} aspect ratio`,
      )
        .withElement(shapeEl(s))
        .withFact("natural", [new Float(round2(nw)), new Float(round2(nh))])
        .withFact("size", [new Float(round2(r.w)), new Float(round2(r.h))])
        .withFact("distortion", new Float(round2(d)))
        .withHelp(
          `use fit=contain or fit=cover${on}, or size (${fmtNum(r.w)}, ${fmtNum(round((r.w * nh) / nw))}) to keep the ratio`,
        ),
    );
  }
}

/// `svg-dropped`: an SVG file lost elements when it was embedded (`<image>`s that are not
/// `data:` URIs, `foreignObject`). Reported once per file, at its first use.
export function svgDropped(ctx: Ctx, out: Sink): void {
  const assets = ctx.scene.assets;
  const seen: number[] = [];
  for (const s of ctx.scene.shapes) {
    for (const img of [s.image, s.icon].filter((i): i is ImageRef => i !== null)) {
      const a = assets[img.asset]!;
      if (a.dropped.length === 0 || seen.includes(img.asset)) {
        continue;
      }
      seen.push(img.asset);
      const lost = a.dropped
        .map(([name, n]) => `${n} \`<${name}>\` element${n === 1 ? "" : "s"}`)
        .join(" and ");
      const dropped = new Map<string, Json>();
      for (const [name, n] of [...a.dropped].sort((x, y) => cmpStr(x[0], y[0]))) {
        dropped.set(name, n);
      }
      out.push(
        Diagnostic.warn("svg-dropped", s.span, `'${a.path}' drops ${lost} when embedded`)
          .withElement(shapeEl(s))
          .withFact("file", a.path)
          .withFact("dropped", dropped)
          .withHelp("embed it as a path, or use a separate `image`"),
      );
    }
  }
}
