import { type PathSeg, fmtNum as n, segFrom, segTo } from "../geom/index.js";
import { type Asset, Fit, type ImageRef, type Scene, type Shape } from "../model/index.js";
import { pathData, strokeAttrs } from "./svg.js";

export function aspect(fit: Fit): string {
  switch (fit) {
    case "contain":
      return "xMidYMid meet";
    case "cover":
      return "xMidYMid slice";
    case "stretch":
      return "none";
  }
}

export function dataUri(a: Asset): string {
  const data = Buffer.from(a.bytes).toString("base64");
  return `data:${a.mime};base64,${data}`;
}

/// Assets drawn more than once are embedded a single time in `<defs>`, with one `<symbol>`
/// per fit used, and drawn with `<use>`.
export class Defs {
  private readonly id_: string;
  readonly assets: readonly Asset[];
  readonly shared: Map<number, Set<string>>;

  constructor(scene: Scene, id: string) {
    const uses = new Map<number, Fit[]>();
    for (const s of scene.shapes) {
      for (const img of [s.image, s.icon]) {
        if (img === null) continue;
        let v = uses.get(img.asset);
        if (v === undefined) {
          v = [];
          uses.set(img.asset, v);
        }
        v.push(img.fit);
      }
    }
    const shared = new Map<number, Set<string>>();
    for (const [i, v] of [...uses].sort(([a], [b]) => a - b)) {
      const a = scene.assets[i];
      if (v.length > 1 && a !== undefined && a.natural[0] > 0.0 && a.natural[1] > 0.0) {
        shared.set(i, new Set(v.map(Fit.name)));
      }
    }
    this.id_ = id;
    this.assets = scene.assets;
    this.shared = shared;
  }

  id(): string {
    return this.id_;
  }

  symbolId(asset: number, fit: Fit): string {
    return `kashika-${this.id_}-asset-${asset}-${Fit.name(fit)}`;
  }

  write(): string {
    if (this.shared.size === 0) {
      return "";
    }
    let out = "<defs>\n";
    for (const [i, fits] of this.shared) {
      const a = this.assets[i]!;
      const [w, h] = [n(a.natural[0]), n(a.natural[1])];
      out += `<image id="kashika-${this.id_}-asset-${i}" width="${w}" height="${h}" href="${dataUri(a)}"/>\n`;
      for (const fit of ["contain", "cover", "stretch"] as const) {
        if (fits.has(Fit.name(fit))) {
          out += `<symbol id="${this.symbolId(i, fit)}" viewBox="0 0 ${w} ${h}" preserveAspectRatio="${aspect(fit)}"><use href="#kashika-${this.id_}-asset-${i}"/></symbol>\n`;
        }
      }
    }
    out += "</defs>\n";
    return out;
  }
}

/// Draws a shape's body (its image, path form or rect), then its icon.
export function body(s: Shape, defs: Defs): string {
  const st = s.style;
  let out = "";
  if (s.image !== null) {
    out += image(s.image, defs);
  } else if (s.form !== null) {
    const d = s.form.draw.filter((sp) => sp.length > 0).map((sp) => subpath(sp));
    if (d.length > 0) {
      out += `<path d="${d.join(" ")}" fill="${st.fill}" stroke-linejoin="round" ${strokeAttrs(st)}/>\n`;
    }
  } else if (s.kind !== "image") {
    const r = s.rect;
    out += `<rect x="${n(r.x)}" y="${n(r.y)}" width="${n(r.w)}" height="${n(r.h)}" rx="${n(s.cornerRadius())}" fill="${st.fill}" ${strokeAttrs(st)}/>\n`;
  }
  if (s.icon !== null) {
    out += image(s.icon, defs);
  }
  return out;
}

export function subpath(sp: readonly PathSeg[]): string {
  let d = pathData(sp);
  const a = sp[0];
  const b = sp[sp.length - 1];
  if (a !== undefined && b !== undefined) {
    if (segFrom(a).dist(segTo(b)) <= 1e-9) {
      d += " Z";
    }
  }
  return d;
}

export function image(img: ImageRef, defs: Defs): string {
  const a = defs.assets[img.asset];
  if (a === undefined) {
    return "";
  }
  const r = img.rect;
  const [x, y, w, h] = [n(r.x), n(r.y), n(r.w), n(r.h)];
  if (defs.shared.has(img.asset)) {
    return `<use href="#${defs.symbolId(img.asset, img.fit)}" x="${x}" y="${y}" width="${w}" height="${h}"/>\n`;
  }
  return `<image x="${x}" y="${y}" width="${w}" height="${h}" preserveAspectRatio="${aspect(img.fit)}" href="${dataUri(a)}"/>\n`;
}
