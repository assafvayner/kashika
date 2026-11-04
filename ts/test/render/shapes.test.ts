import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test } from "vitest";

import { Options, compile, compileWith } from "../../src/model/index.js";
import { renderSvg, svgToPng } from "../../src/render/index.js";
import { Pixmap, redPng } from "./support.js";

const dirs: string[] = [];

afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

function tempdir(): string {
  const d = mkdtempSync(join(tmpdir(), "kashika-render-"));
  dirs.push(d);
  return d;
}

function svg(src: string): string {
  const [scene, diags] = compile(src);
  expect(diags).toEqual([]);
  return renderSvg(scene);
}

function svgIn(dir: string, src: string): string {
  const [scene, diags] = compileWith(src, new Options({ path: join(dir, "main.kash") }));
  expect(diags).toEqual([]);
  return renderSvg(scene);
}

function pngHeader(w: number, h: number): Uint8Array {
  const b = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52];
  const be = (v: number) => [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
  b.push(...be(w), ...be(h), 8, 6, 0, 0, 0, 0, 0, 0, 0);
  return new Uint8Array(b);
}

function count(s: string, needle: string): number {
  return s.split(needle).length - 1;
}

function find(s: string, needle: string): number {
  const i = s.indexOf(needle);
  if (i < 0) throw new Error(`${needle} not found in ${s}`);
  return i;
}

async function png(s: string): Promise<Pixmap> {
  const r = await svgToPng(s);
  if (!r.ok) throw new Error(r.error);
  return Pixmap.decodePng(r.value);
}

test("diamond_draws_a_closed_path", () => {
  const s = svg('diagram "d" size (200,100)\nbox d "D" at (0,0) size (120,80) shape=diamond\n');
  expect(s).toContain(
    '<path d="M 60 0 L 120 40 L 60 80 L 0 40 L 60 0 Z" fill="#ffffff" stroke-linejoin="round" stroke="#334155" stroke-width="1.5"/>',
  );
  expect(s).not.toContain('<rect x="0" y="0" width="120"');
});

test("cylinder_draws_arcs_as_cubics", () => {
  const s = svg(
    'diagram "c" size (200,100)\nbox c "DB" at (0,0) size (120,80) shape=cylinder style=datastore\n',
  );
  expect(s).toContain('<path d="M 0 20 C ');
  expect(s).toContain('fill="#e6f4ea"');
});

test("images_and_icons_are_embedded", () => {
  const dir = tempdir();
  writeFileSync(join(dir, "logo.png"), pngHeader(200, 100));
  const s = svgIn(
    dir,
    'diagram "i" size (300,200)\nimage logo "logo.png" at (20,20) size (80,80) fit=cover\n' +
      'box a "A" at (120,20) size (160,60) icon="server"\n',
  );
  expect(s).toContain(
    '<image x="20" y="20" width="80" height="80" preserveAspectRatio="xMidYMid slice" href="data:image/png;base64,iVBORw0KGgo',
  );
  const rect = find(s, '<rect x="120" y="20"');
  const icon = find(
    s,
    '<image x="128" y="38" width="24" height="24" preserveAspectRatio="xMidYMid meet" href="data:image/svg+xml;base64,',
  );
  expect(rect).toBeLessThan(icon);
  expect(s).not.toContain("<script");
});

test("svg_shapes_draw_only_their_image", () => {
  const dir = tempdir();
  writeFileSync(
    join(dir, "t.svg"),
    '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><path d="M 0 0 L 40 20"/></svg>',
  );
  const s = svgIn(
    dir,
    'diagram "s" size (300,200)\nshape t from "t.svg" text-area=below\nbox a at (100,100) size (80,40) shape=t\n',
  );
  expect(s).toContain(
    '<image x="100" y="100" width="80" height="40" preserveAspectRatio="xMidYMid meet"',
  );
  expect(s).not.toContain('<rect x="100" y="100"');
});

test("image_paint_order_follows_file_order_with_boxes", () => {
  const dir = tempdir();
  writeFileSync(join(dir, "logo.png"), pngHeader(4, 4));
  const s = svgIn(
    dir,
    'diagram "o" size (300,200)\nbox a at (0,0) size (50,50)\nimage i "logo.png" at (60,0) size (50,50)\n' +
      "box b at (120,0) size (50,50)\nline l a.bottom -> v 20\n",
  );
  const a = find(s, '<rect x="0" y="0" width="50"');
  const i = find(s, "<image");
  const b = find(s, '<rect x="120"');
  const l = find(s, '<path d="M 25 50');
  expect(a < i && i < b && b < l).toBe(true);
});

test("output_is_deterministic", () => {
  const dir = tempdir();
  writeFileSync(join(dir, "logo.png"), pngHeader(4, 4));
  const src =
    'diagram "o" size (300,200)\nimage i "logo.png" at (60,0) size (50,50)\nbox c at (0,60) size (90,60) shape=cloud icon="lock"\n';
  expect(svgIn(dir, src)).toBe(svgIn(dir, src));
});

test("png_output_draws_embedded_images", async () => {
  const dir = tempdir();
  writeFileSync(join(dir, "red.png"), redPng(8));
  const s = svgIn(dir, 'diagram "p" size (100,100)\nimage r "red.png" at (10,10) size (40,40)\n');
  const pm = await png(s);
  const [r, g] = pm.pixel(30, 30);
  expect([r, g]).toEqual([255, 0]);
  expect(pm.pixel(80, 80)[3]).toBe(0);
});

test("repeated_images_embed_once_and_still_rasterize", async () => {
  const dir = tempdir();
  writeFileSync(join(dir, "red.png"), redPng(8));
  const s = svgIn(
    dir,
    'diagram "p" size (100,100)\nimage a "red.png" at (10,10) size (30,30)\nimage b "red.png" at (60,60) size (30,30) fit=cover\n',
  );
  expect(count(s, "data:image/png;base64,")).toBe(1);
  const pm = await png(s);
  expect(pm.pixel(20, 20)[0]).toBe(255);
  expect(pm.pixel(75, 75)[0]).toBe(255);
  expect(pm.pixel(50, 50)[3]).toBe(0);
});
