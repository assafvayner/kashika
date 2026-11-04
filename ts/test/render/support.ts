import { Resvg } from "@resvg/resvg-js";

/// A rasterized image as premultiplied RGBA, like a tiny-skia `Pixmap`.
export class Pixmap {
  constructor(
    readonly width: number,
    readonly height: number,
    readonly data: Uint8Array,
  ) {}

  /// Decodes a PNG, JPEG or WebP by drawing it at its natural size with resvg.
  static decode(bytes: Uint8Array, mime: string, width: number, height: number): Pixmap {
    const b64 = Buffer.from(bytes).toString("base64");
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><image width="${width}" height="${height}" href="data:${mime};base64,${b64}"/></svg>`;
    return Pixmap.render(svg);
  }

  static decodePng(png: Uint8Array): Pixmap {
    const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
    return Pixmap.decode(png, "image/png", view.getUint32(16), view.getUint32(20));
  }

  static render(svg: string): Pixmap {
    const img = new Resvg(svg, { font: { loadSystemFonts: false } }).render();
    return new Pixmap(img.width, img.height, new Uint8Array(img.pixels));
  }

  /// Premultiplied RGBA at (x, y).
  pixel(x: number, y: number): [number, number, number, number] {
    const i = (y * this.width + x) * 4;
    const d = this.data;
    return [d[i]!, d[i + 1]!, d[i + 2]!, d[i + 3]!];
  }

  /// Straight RGBA at (x, y), like tiny-skia's `demultiply`.
  rgba(x: number, y: number): [number, number, number, number] {
    const [r, g, b, a] = this.pixel(x, y);
    if (a === 0) return [0, 0, 0, 0];
    const de = (c: number) => Math.round((c * 255) / a);
    return [de(r), de(g), de(b), a];
  }
}

/// A solid red `size`×`size` PNG.
export function redPng(size: number): Uint8Array {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}"><rect width="${size}" height="${size}" fill="#ff0000"/></svg>`;
  return new Uint8Array(new Resvg(svg, { font: { loadSystemFonts: false } }).render().asPng());
}
