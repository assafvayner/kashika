import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "vitest";

import { type Result } from "../../src/diag/rust.js";
import { MAX_RASTER_SIDE, Options, compile, compileWith } from "../../src/model/index.js";
import {
  checkRasterSize,
  renderSvg,
  svgToJpeg,
  svgToPng,
  svgToWebp,
} from "../../src/render/index.js";
import { Pixmap, redPng } from "./support.js";

function unwrap<T>(r: Result<T, string>): T {
  if (!r.ok) throw new Error(r.error);
  return r.value;
}

function unwrapErr<T>(r: Result<T, string>): string {
  if (r.ok) throw new Error("expected an error");
  return r.error;
}

test("accepts_the_cap", () => {
  expect(checkRasterSize(MAX_RASTER_SIDE, 6103).ok).toBe(true);
  expect(checkRasterSize(10_000, 10_000).ok).toBe(true);
  expect(checkRasterSize(1, 1).ok).toBe(true);
});

test("rejects_an_oversized_side", () => {
  const e = unwrapErr(checkRasterSize(MAX_RASTER_SIDE + 1, 10));
  expect(e.includes("16384") && e.includes("svg")).toBe(true);
  expect(checkRasterSize(10, MAX_RASTER_SIDE + 1).ok).toBe(false);
});

test("rejects_an_oversized_area", () => {
  expect(checkRasterSize(MAX_RASTER_SIDE, MAX_RASTER_SIDE).ok).toBe(false);
  expect(checkRasterSize(10_001, 10_000).ok).toBe(false);
});

test("rasterize_rejects_before_allocating", async () => {
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="1000000" height="1000000"/>';
  expect(unwrapErr(await svgToPng(svg))).toContain("too large");
});

const BASIC =
  'diagram "Basic" size (480,240)\n' +
  'box api "API" at (40,40) size (120,60) style=service\n' +
  'box db "DB" at api.right+(160,0) size (120,60) pin=left\n' +
  "line api_db api.right -> db.left end=arrow\n";

test("png_has_signature_and_canvas_size", async () => {
  const [scene, diags] = compile(BASIC);
  expect(diags).toEqual([]);
  const png = unwrap(await svgToPng(renderSvg(scene)));
  expect([...png.slice(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  expect([view.getUint32(16), view.getUint32(20)]).toEqual([480, 240]);
});

test("invalid_svg_is_an_error", async () => {
  expect((await svgToPng("<not svg")).ok).toBe(false);
});

test("png_is_deterministic_and_draws_glyph_paths_without_fonts", async () => {
  const [scene] = compile(BASIC);
  const svg = renderSvg(scene);
  const a = unwrap(await svgToPng(svg));
  expect(a).toEqual(unwrap(await svgToPng(svg)));
  const pix = Pixmap.decodePng(a);
  let dark = false;
  for (let x = 60; x < 140 && !dark; x++) {
    for (let y = 50; y < 90 && !dark; y++) {
      if (pix.pixel(x, y)[0] < 128) dark = true;
    }
  }
  expect(dark).toBe(true);
});

test("png_rasterizes_embedded_images", async () => {
  const dir = mkdtempSync(join(tmpdir(), "kashika-render-"));
  try {
    writeFileSync(join(dir, "red.png"), redPng(10));
    const opts = new Options({ path: join(dir, "i.kash") });
    const src = 'diagram "i" size (100,100)\nimage logo "red.png" at (20,20) size (40,40)\n';
    const [scene, diags] = compileWith(src, opts);
    expect(diags).toEqual([]);
    const pix = Pixmap.decodePng(unwrap(await svgToPng(renderSvg(scene))));
    const p = pix.pixel(40, 40);
    expect(p[0] > 200 && p[1] < 50).toBe(true);
    expect(pix.pixel(80, 80)[3]).toBe(0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

const LABELED =
  'diagram "Label" size (300,100)\n' +
  'line l (20,50) -> (280,50) label="passive open" stroke=#dc2626 stroke-width=3\n';

test("line_label_masks_the_line_between_words", async () => {
  const [scene, diags] = compile(LABELED);
  expect(diags).toEqual([]);
  const rect = scene.lines[0]!.labels[0]!.rect;
  const svg = renderSvg(scene);
  const r = (v: number) => Math.round(v * 100) / 100;
  const mask = `<rect x="${r(rect.x)}" y="${r(rect.y)}" width="${r(rect.w)}" height="${r(rect.h)}" fill="#000"/>`;
  expect(svg).toContain(mask);

  const pix = Pixmap.decodePng(unwrap(await svgToPng(svg)));
  const isLine = (x: number, y: number) => {
    const p = pix.pixel(x, y);
    return p[0] > 150 && p[1] < 100;
  };
  expect(isLine(25, 50)).toBe(true);
  const [x0, x1] = [Math.ceil(rect.x) + 1, Math.floor(rect.x + rect.w) - 1];
  for (let x = x0; x <= x1; x++) {
    expect(isLine(x, 50), `line shows through the label at x=${x}`).toBe(false);
  }
});

const TRANSPARENT =
  '<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2">' +
  '<rect width="2" height="2" fill="#ff0000"/></svg>';

test("webp_keeps_alpha", async () => {
  expect(unwrapErr(await svgToWebp(TRANSPARENT))).toBe(
    "webp output is not supported by the TypeScript build",
  );
});

test("jpeg_composites_transparency_onto_white", async () => {
  const empty = '<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2"/>';
  const white =
    '<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2">' +
    '<rect width="4" height="2" fill="#ffffff"/></svg>';
  const a = unwrap(await svgToJpeg(empty));
  expect([...a.slice(0, 3)]).toEqual([0xff, 0xd8, 0xff]);
  expect(a).toEqual(unwrap(await svgToJpeg(white)));
});

const BARE = 'diagram "Bare" size (100,60)\nbox a at (10,10) size (30,30)\n';
const DARK = 'diagram "Dark" size (100,60) background=#0f172a\nbox a at (10,10) size (30,30)\n';

type Encode = (svg: string) => Promise<Result<Uint8Array, string>>;

async function raster(src: string, encode: Encode): Promise<Pixmap> {
  const [scene, diags] = compile(src);
  expect(diags).toEqual([]);
  const bytes = unwrap(await encode(renderSvg(scene)));
  const mime = bytes[0] === 0x89 ? "png" : bytes[0] === 0xff ? "jpeg" : "webp";
  const [w, h] = scene.size;
  return Pixmap.decode(bytes, `image/${mime}`, w, h);
}

function near(a: number[], b: number[]): boolean {
  return a.every((x, i) => Math.abs(x - b[i]!) <= 3);
}

test("png_canvas_is_transparent_by_default_and_takes_the_background", async () => {
  expect((await raster(BARE, svgToPng)).rgba(80, 50)[3]).toBe(0);
  const p = (await raster(DARK, svgToPng)).rgba(80, 50);
  expect(near(p, [0x0f, 0x17, 0x2a, 255]), `${p}`).toBe(true);
});

test("webp_canvas_keeps_transparency", async () => {
  const [scene] = compile(BARE);
  expect((await svgToWebp(renderSvg(scene))).ok).toBe(false);
});

test("jpeg_canvas_is_white_by_default_and_takes_the_background", async () => {
  let p = (await raster(BARE, svgToJpeg)).rgba(80, 50);
  expect(near(p, [255, 255, 255, 255]), `${p}`).toBe(true);
  p = (await raster(DARK, svgToJpeg)).rgba(80, 50);
  expect(near(p, [0x0f, 0x17, 0x2a, 255]), `${p}`).toBe(true);
});

const ON_GROUP =
  'diagram "g" size (300,100)\n' +
  "group g at (0,0) size (300,100) fill=#fde68a stroke=#000000 {\n" +
  '  line l (20,50) -> (280,50) label="passive open" stroke=#dc2626 stroke-width=3\n' +
  "}\n";

test("line_label_on_a_filled_group_shows_the_group_fill", async () => {
  const [scene, diags] = compile(ON_GROUP);
  expect(diags).toEqual([]);
  const lab = scene.lines[0]!.labels[0]!;
  const pix = await raster(ON_GROUP, svgToPng);
  const xs: number[] = [];
  for (let x = Math.ceil(lab.rect.x) + 1; x < Math.floor(lab.rect.x + lab.rect.w); x++) xs.push(x);
  expect(xs.some((x) => near(pix.rgba(x, 50), [0xfd, 0xe6, 0x8a, 255]))).toBe(true);
  for (const x of xs) {
    const p = pix.rgba(x, 50);
    expect(near(p, [0xdc, 0x26, 0x26, 255]), `line shows through the label at x=${x}`).toBe(false);
    expect(near(p, [255, 255, 255, 255]), `white mask under the label at x=${x}`).toBe(false);
  }
  expect(near(pix.rgba(25, 50), [0xdc, 0x26, 0x26, 255])).toBe(true);
});
