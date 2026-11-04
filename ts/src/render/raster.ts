import { type Result, err, ok } from "../diag/rust.js";
import { MAX_RASTER_PIXELS, MAX_RASTER_SIDE } from "../model/index.js";

type ResvgModule = typeof import("@resvg/resvg-js");
type JpegModule = typeof import("jpeg-js");

interface Pixmap {
  width: number;
  height: number;
  /// Premultiplied RGBA.
  pixels: Uint8Array;
  png: () => Uint8Array;
}

export function checkRasterSize(width: number, height: number): Result<null, string> {
  if (width > MAX_RASTER_SIDE || height > MAX_RASTER_SIDE || width * height > MAX_RASTER_PIXELS) {
    return err(
      `the ${width}x${height} canvas is too large for raster output (at most ${MAX_RASTER_SIDE} px per side and ${MAX_RASTER_PIXELS} px in total); use .svg output or a smaller \`size\``,
    );
  }
  return ok(null);
}

async function optional<T>(name: string, load: () => Promise<T>): Promise<Result<T, string>> {
  try {
    return ok(await load());
  } catch {
    return err(`raster output needs the optional dependency \`${name}\`; install it or use .svg output`);
  }
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

async function rasterize(svg: string): Promise<Result<Pixmap, string>> {
  const resvg = await optional<ResvgModule>("@resvg/resvg-js", () => import("@resvg/resvg-js"));
  if (!resvg.ok) return resvg;
  let tree: InstanceType<ResvgModule["Resvg"]>;
  try {
    tree = new resvg.value.Resvg(svg, { font: { loadSystemFonts: false }, logLevel: "off" });
  } catch (e) {
    return err(message(e));
  }
  const [w, h] = [Math.round(tree.width), Math.round(tree.height)];
  const size = checkRasterSize(w, h);
  if (!size.ok) return size;
  if (w === 0 || h === 0) return err("the canvas is empty");
  let image: ReturnType<typeof tree.render>;
  try {
    image = tree.render();
  } catch (e) {
    return err(message(e));
  }
  return ok({
    width: image.width,
    height: image.height,
    pixels: new Uint8Array(image.pixels),
    png: () => new Uint8Array(image.asPng()),
  });
}

/// Rasterizes an SVG document at 1 px per diagram unit. Kashika draws text as glyph paths,
/// so no fonts are loaded.
export async function svgToPng(svg: string): Promise<Result<Uint8Array, string>> {
  const pixmap = await rasterize(svg);
  if (!pixmap.ok) return pixmap;
  try {
    return ok(pixmap.value.png());
  } catch (e) {
    return err(message(e));
  }
}

const JPEG_QUALITY = 90;

/// Rasterizes like `svgToPng`, composited onto white because JPEG has no alpha.
export async function svgToJpeg(svg: string): Promise<Result<Uint8Array, string>> {
  const pixmap = await rasterize(svg);
  if (!pixmap.ok) return pixmap;
  const jpeg = await optional<JpegModule>("jpeg-js", async () => {
    const m = (await import("jpeg-js")) as JpegModule & { default?: JpegModule };
    return m.default ?? m;
  });
  if (!jpeg.ok) return jpeg;
  const { width, height, pixels } = pixmap.value;
  const rgba = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    const inv = 255 - pixels[i * 4 + 3]!;
    for (let c = 0; c < 3; c++) {
      rgba[i * 4 + c] = Math.min(pixels[i * 4 + c]! + inv, 255);
    }
    rgba[i * 4 + 3] = 255;
  }
  try {
    const out = jpeg.value.encode({ width, height, data: rgba }, JPEG_QUALITY);
    return ok(new Uint8Array(out.data));
  } catch (e) {
    return err(message(e));
  }
}

/// WebP needs an encoder the TypeScript build does not bundle.
export async function svgToWebp(_svg: string): Promise<Result<Uint8Array, string>> {
  return err("webp output is not supported by the TypeScript build");
}
