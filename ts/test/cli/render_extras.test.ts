import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "vitest";

import { fixture, jpegSize, kashika, pngSize, tempdir } from "../support/cli.js";

const UNSUPPORTED_WEBP = "webp output is not supported by the TypeScript build";

test("render_writes_png", () => {
  const dir = tempdir();
  const out = join(dir, "out.png");
  const r = kashika(["render", fixture("basic.kash"), "-o", out]);
  expect(r.code).toBe(0);
  expect(r.stdout).toContain("output: written");
  expect([...readFileSync(out).subarray(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
});

test("force_draws_red_markers_at_errors", () => {
  const dir = tempdir();
  const out = join(dir, "out.svg");
  expect(kashika(["render", fixture("typo.kash"), "-o", out, "--force"]).code).toBe(2);
  const svg = readFileSync(out, "utf8");
  expect(svg).toContain('<g id="kashika-overlay">');
  expect(svg).toContain("error[unknown-anchor]");
});

test("clean_render_has_no_overlay_and_debug_adds_one", () => {
  const dir = tempdir();
  const plain = join(dir, "plain.svg");
  const debug = join(dir, "debug.svg");
  expect(kashika(["render", fixture("basic.kash"), "-o", plain]).code).toBe(0);
  expect(kashika(["render", fixture("basic.kash"), "-o", debug, "--debug"]).code).toBe(0);
  expect(readFileSync(plain, "utf8")).not.toContain("kashika-overlay");
  expect(readFileSync(debug, "utf8")).toContain("kashika-overlay");
});

test("debug_marks_lint_warnings_at_their_geometry", () => {
  const dir = tempdir();
  const out = join(dir, "debug.svg");
  expect(kashika(["render", fixture("crossing.kash"), "-o", out, "--debug"]).code).toBe(1);
  const svg = readFileSync(out, "utf8");
  expect(svg).toContain('<circle cx="200" cy="150" r="6" fill="none" stroke="#ea580c"');
  expect(svg).toContain('aria-label="warn[cross]"');
  expect(svg, "suppressed findings are not marked").not.toContain('<circle cx="240" cy="150"');
});

function renderTo(ext: string, extra: string[] = []): Uint8Array {
  const dir = tempdir();
  const out = join(dir, `out.${ext}`);
  const r = kashika(["render", fixture("basic.kash"), "-o", out, ...extra]);
  expect(r.code, r.stderr).toBe(0);
  return new Uint8Array(readFileSync(out));
}

/// WebP needs an encoder the TypeScript build does not bundle: `render` fails with a tool
/// error and writes nothing.
function webpFails(extra: string[] = []): void {
  const dir = tempdir();
  const out = join(dir, "out.webp");
  const r = kashika(["render", fixture("basic.kash"), "-o", out, ...extra]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain(UNSUPPORTED_WEBP);
  expect(existsSync(out)).toBe(false);
}

test("render_writes_jpeg_and_webp_with_diagram_size", () => {
  const expected = pngSize(renderTo("png"));
  for (const ext of ["jpg", "jpeg"]) {
    const b = renderTo(ext);
    expect([...b.subarray(0, 3)]).toEqual([0xff, 0xd8, 0xff]);
    expect(jpegSize(b)).toEqual(expected);
  }
  webpFails();
});

test("jpeg_and_webp_are_deterministic", () => {
  expect(renderTo("jpg")).toEqual(renderTo("jpg"));
  webpFails();
});

test("debug_overlay_applies_to_jpeg_and_webp", () => {
  expect(renderTo("jpg")).not.toEqual(renderTo("jpg", ["--debug"]));
  webpFails(["--debug"]);
});

test("unsupported_extension_lists_supported_ones", () => {
  const r = kashika(["render", fixture("basic.kash"), "-o", "x.gif"]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("output must end in .svg, .png, .jpg, .jpeg or .webp");
});

test("oversized_raster_canvas_is_a_tool_error_but_svg_is_fine", () => {
  const dir = tempdir();
  const src = join(dir, "big.kash");
  writeFileSync(src, 'diagram "big" size (20000,100)\n');
  const png = join(dir, "out.png");
  const r = kashika(["render", src, "-o", png]);
  expect(r.code).toBe(3);
  expect(r.stderr).toContain("too large for raster output");
  expect(existsSync(png)).toBe(false);
  expect(kashika(["render", src, "-o", join(dir, "out.svg")]).code).toBe(0);
});
