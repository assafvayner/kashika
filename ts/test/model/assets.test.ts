import { describe, expect, test } from "vitest";

import { totalCmp } from "../../src/diag/rust.js";
import { Pt } from "../../src/geom/index.js";
import { checkDepth, decode, probe } from "../../src/model/assets.js";
import { BUNDLED_ICONS, bundled } from "../../src/model/icons.js";
import { type Asset } from "../../src/model/scene.js";

const EVIL = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10" onload="alert(1)"><script>alert(2)</script><foreignObject width="10" height="10"><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject><image width="5" height="5" xlink:href="https://example.com/x.png"/><image width="5" height="5" href="secret.png"/><rect width="10" height="10" fill="#f00"/></svg>`;

const enc = (s: string) => new TextEncoder().encode(s);
const dec = (b: Uint8Array) => new TextDecoder().decode(b);

function png(w: number, h: number): Uint8Array {
  const b = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52];
  const be = (v: number) => [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
  return new Uint8Array([...b, ...be(w), ...be(h), 8, 6, 0, 0, 0, 0, 0, 0, 0]);
}

function unwrap(r: { ok: true; value: Asset } | { ok: false; error: string }): Asset {
  if (!r.ok) throw new Error(r.error);
  return r.value;
}

function unwrapErr(r: { ok: true; value: Asset } | { ok: false; error: string }): string {
  if (r.ok) throw new Error("expected an error");
  return r.error;
}

function b64(b: Uint8Array): string {
  return Buffer.from(b).toString("base64");
}

function nested(depth: number): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">${"<g>".repeat(depth - 1)}<rect width="10" height="10"/>${"</g>".repeat(depth - 1)}</svg>`;
}

describe("assets", () => {
  test("svg_is_sanitized", () => {
    const a = unwrap(decode("evil.svg", enc(EVIL), true, null));
    const out = dec(a.bytes);
    for (const bad of ["script", "alert", "foreignObject", "example.com", "secret.png", "onload"]) {
      expect(out.includes(bad), `\`${bad}\` survived: ${out}`).toBe(false);
    }
    expect(out.includes("<path"), out).toBe(true);
    expect(a.mime).toBe("image/svg+xml");
    expect(a.natural).toEqual([10.0, 10.0]);
    expect(a.dropped).toEqual([
      ["image", 2],
      ["foreignObject", 1],
    ]);
  });

  test("svg_text_is_drawn_as_paths_in_the_bundled_fonts", () => {
    const text = (family: string) => {
      const src = `<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><text x="2" y="15" font-size="12"${family}>Hello</text></svg>`;
      const a = unwrap(decode("t.svg", enc(src), true, null));
      expect(a.dropped.length).toBe(0);
      return dec(a.bytes);
    };
    const inter = text("");
    expect(inter.includes("<text"), inter).toBe(false);
    expect(inter.includes("<path"), inter).toBe(true);
    expect(text(` font-family="Inter"`)).toBe(inter);
    expect(text(` font-family="Helvetica, sans-serif"`)).toBe(inter);
    expect(text(` font-family="Times"`)).toBe(inter);
    const mono = text(` font-family="monospace"`);
    expect(mono).not.toBe(inter);
    expect(text(` font-family="JetBrains Mono"`)).toBe(mono);
    expect(text(` font-family="Courier New, monospace"`)).toBe(mono);
  });

  test("svg_hull_is_in_unit_coordinates", () => {
    const src = `<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path d="M 10 0 L 30 0 L 40 20 L 0 20 Z"/></svg>`;
    const a = unwrap(decode("t.svg", enc(src), true, null));
    expect(a.natural).toEqual([40.0, 20.0]);
    const h = [...a.unitHull].sort((p, q) => totalCmp(p.x, q.x) || totalCmp(p.y, q.y));
    expect(h).toEqual([new Pt(0.0, 1.0), new Pt(0.25, 0.0), new Pt(0.75, 0.0), new Pt(1.0, 1.0)]);
  });

  test("current_color_resolves_to_the_given_color", () => {
    const svg = (root: string, paint: string) =>
      `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"${root}><rect width="10" height="10" fill="${paint}"/></svg>`;
    let a = unwrap(decode("c.svg", enc(svg("", "currentColor")), true, "#ffffff"));
    expect(a.currentColor).toBe("#ffffff");
    const out = dec(a.bytes);
    expect(out.includes('fill="#ffffff"'), out).toBe(true);
    for (const [root, paint] of [
      ["", "#123456"],
      [' color="#123456"', "currentColor"],
    ] as const) {
      a = unwrap(decode("c.svg", enc(svg(root, paint)), true, "#ffffff"));
      expect(a.currentColor, `${root} ${paint}`).toBe(null);
      expect(dec(a.bytes).includes("#123456")).toBe(true);
    }
    a = unwrap(decode("c.svg", enc(svg("", "currentColor")), true, null));
    expect(a.currentColor).toBe(null);
    expect(dec(a.bytes).includes("#000000")).toBe(true);
  });

  test("raster_sizes_come_from_the_header", () => {
    const a = unwrap(decode("x.png", png(200, 100), false, null));
    expect([a.mime, a.natural]).toEqual(["image/png", [200.0, 100.0]]);
    expect(a.bytes).toEqual(png(200, 100));
    expect(a.unitHull.length).toBe(4);
    const e = unwrapErr(
      decode("x.gif", new Uint8Array([...enc("GIF89a"), 0x01, 0x00, 0x01, 0x00]), false, null),
    );
    expect(e.includes("only SVG, PNG and JPEG"), e).toBe(true);
    expect(decode("bad.svg", enc("<svg"), true, null).ok).toBe(false);
  });

  test("bundled_icons_decode", () => {
    for (const name of BUNDLED_ICONS) {
      const src = bundled(name)![1];
      const a = unwrap(decode(`builtin:${name}`, enc(src), true, null));
      expect(a.natural, name).toEqual([24.0, 24.0]);
      expect(a.unitHull.length >= 3, name).toBe(true);
    }
  });

  test("oversized_images_are_refused_from_the_header", () => {
    let e = unwrapErr(decode("x.png", png(100_000, 10), false, null));
    expect(e.includes("100000x10") && e.includes("too large"), e).toBe(true);
    e = unwrapErr(decode("x.png", png(8_000, 8_000), false, null));
    expect(e.includes("too large"), e).toBe(true);
    expect(decode("x.png", png(8_192, 3_906), false, null).ok).toBe(true);
  });

  test("deep_nesting_is_refused_before_parsing", () => {
    let e = unwrapErr(decode("d.svg", enc(nested(20_000)), true, null));
    expect(e.includes("nested more than 256 deep"), e).toBe(true);
    expect(decode("d.svg", enc(nested(256)), true, null).ok).toBe(true);
    e = unwrapErr(decode("d.svg", enc(nested(257)), true, null));
    expect(e.includes("nested more than 256 deep"), e).toBe(true);
    const flat = `<?xml version="1.0"?><!-- <g><g> --><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">${`<g a='>'><rect width="1" height="1"/></g>`.repeat(300)}<![CDATA[<g>]]></svg>`;
    expect(checkDepth(enc(flat)).ok).toBe(true);
  });

  test("deep_nesting_in_an_embedded_svg_is_refused", () => {
    const src = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="data:image/svg+xml;base64,${b64(enc(nested(20_000)))}"/></svg>`;
    const e = unwrapErr(decode("n.svg", enc(src), true, null));
    expect(e.includes("embedded SVG nests elements more than 256 deep"), e).toBe(true);
  });

  test("oversized_images_nested_in_svg_are_refused", () => {
    const src = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="10" height="10" href="data:image/png;base64,${b64(png(100_000, 10))}"/></svg>`;
    const e = unwrapErr(decode("n.svg", enc(src), true, null));
    expect(e.includes("too large"), e).toBe(true);
  });

  test("probe_reports_natural_size", () => {
    expect(probe(png(3, 7), false)).toEqual({ ok: true, value: [3.0, 7.0] });
  });
});
