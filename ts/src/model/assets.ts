import { type Result, err, ok } from "../diag/rust.js";
import { type PathSeg, Pt, arcToCubics, convexHull } from "../geom/index.js";
import { ICON_SENTINEL_COLOR, bundledIcon } from "./icons.js";
import { FileError, type Found, type Loader } from "./loader.js";
import { Asset } from "./scene.js";
import { ICON_COLOR } from "./style.js";
import { outline, runWidth } from "../text/font.js";
import { Run } from "../text/index.js";
import {
  type XmlElement,
  type XmlNode,
  attr,
  descendants,
  parseXml,
  serializeXml,
} from "./xml.js";

/// Largest image or SVG file read.
export const MAX_BYTES = 16 * 1024 * 1024;
/// Deepest element nesting accepted in an SVG, judged from the source before parsing.
export const MAX_SVG_DEPTH = 256;

/// Raster output (PNG, JPEG, WebP) is 1 px per diagram unit; a canvas beyond either limit is refused.
export const MAX_RASTER_SIDE = 16_384;
export const MAX_RASTER_PIXELS = 100_000_000;
/// Attached PNG/JPEG images, judged by their header before any pixel is decoded.
export const MAX_IMAGE_SIDE = 8_192;
export const MAX_IMAGE_PIXELS = 32_000_000;

export function checkImageSize(width: number, height: number): Result<null, string> {
  if (width > MAX_IMAGE_SIDE || height > MAX_IMAGE_SIDE || width * height > MAX_IMAGE_PIXELS) {
    return err(
      `the ${width}x${height} image is too large (at most ${MAX_IMAGE_SIDE} px per side and ${MAX_IMAGE_PIXELS} px in total)`,
    );
  }
  return ok(null);
}

/// Loaded assets, deduplicated by canonical path, in first-use order.
export class Store {
  private assets: Asset[] = [];
  private keys: string[] = [];

  static default(): Store {
    return new Store();
  }

  /// Finds `name` from the directory `from`, loads it once and returns its index.
  load(loader: Loader, from: string, name: string): Result<number, FileError> {
    const found = loader.find(from, name);
    if (!found.ok) return found;
    return this.loadFound(loader, found.value, name, null);
  }

  /// Like `load`, for an icon: an SVG that uses `currentColor` and sets no `color` on its root
  /// is drawn in `color`, and is kept apart from the same file loaded in another color.
  loadIcon(loader: Loader, from: string, name: string, color: string): Result<number, FileError> {
    const found = loader.find(from, name);
    if (!found.ok) return found;
    return this.loadFound(loader, found.value, name, color);
  }

  /// Like `load`, but only accepts SVG files; anything else is refused before it is read.
  loadSvg(loader: Loader, from: string, name: string, what: string): Result<number, FileError> {
    const found = loader.find(from, name);
    if (!found.ok) return found;
    if (found.value.kind === "file" && !isSvgPath(found.value.path)) {
      return err(new FileError(`${what} needs an SVG file; \`${name}\` is not one`));
    }
    return this.loadFound(loader, found.value, name, null);
  }

  private loadFound(
    loader: Loader,
    found: Found,
    name: string,
    color: string | null,
  ): Result<number, FileError> {
    const [key, c] =
      found.kind === "file"
        ? [found.path, color]
        : [`builtin:${found.name}`, color ?? ICON_COLOR];
    const colored = `${key}\u0000${c ?? ""}`;
    const i = this.keys.findIndex((k) => k === key || k === colored);
    if (i >= 0) {
      return ok(i);
    }
    const bytes = loader.read(found, name, MAX_BYTES);
    if (!bytes.ok) return bytes;
    const [isSvg, shown] =
      found.kind === "file" ? [isSvgPath(found.path), name] : [true, key];
    const asset = decode(shown, bytes.value, isSvg, c);
    if (!asset.ok) return err(new FileError(`cannot use \`${name}\`: ${asset.error}`));
    const recolorable = isSvg && usesCurrentColor(bytes.value);
    this.keys.push(recolorable ? colored : key);
    this.assets.push(asset.value);
    return ok(this.assets.length - 1);
  }

  get(i: number): Asset {
    return this.assets[i]!;
  }

  intoVec(): Asset[] {
    return this.assets;
  }
}

function isSvgPath(p: string): boolean {
  const base = p.slice(Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\")) + 1);
  const dot = base.lastIndexOf(".");
  return dot > 0 && base.slice(dot + 1).toLowerCase() === "svg";
}

/// The natural size of an image file's contents, for `kashika icons`.
export function probe(bytes: Uint8Array, isSvg: boolean): Result<[number, number], string> {
  const a = decode("", bytes, isSvg, null);
  return a.ok ? ok(a.value.natural) : a;
}

/// Decodes a bundled icon as the renderer does.
export function decodeBundledIcon(name: string, color: string): Result<Asset, string> {
  const icon = bundledIcon(name);
  if (icon === null) return err(`no bundled icon ${name}`);
  return decode(`builtin:${name}`, new TextEncoder().encode(icon.source), true, color);
}

export function decode(
  path: string,
  bytes: Uint8Array,
  isSvg: boolean,
  color: string | null,
): Result<Asset, string> {
  if (isSvg) {
    return svg(path, bytes, color);
  } else {
    return raster(path, bytes);
  }
}

function unitSquare(): Pt[] {
  return [new Pt(0.0, 0.0), new Pt(1.0, 0.0), new Pt(1.0, 1.0), new Pt(0.0, 1.0)];
}

function raster(path: string, bytes: Uint8Array): Result<Asset, string> {
  const t = imageType(bytes);
  let mime: string;
  if (t === "png") {
    mime = "image/png";
  } else if (t === "jpeg") {
    mime = "image/jpeg";
  } else {
    return err("only SVG, PNG and JPEG images are supported");
  }
  const size = blobSize(bytes)!;
  if (!size.ok) return err(`cannot read the image size: ${size.error}`);
  const [w, h] = size.value;
  const c = checkImageSize(w, h);
  if (!c.ok) return c;
  return ok(
    new Asset({
      path,
      mime,
      natural: [w, h],
      bytes: bytes.slice(),
      unitHull: unitSquare(),
      currentColor: null,
      dropped: [],
    }),
  );
}

type ImageType = "jpeg" | "png" | "gif" | "webp" | "other";

/// The format of an image from its first 12 bytes, like `imagesize::image_type`, for the
/// formats the renderer distinguishes; `null` when there are fewer than 12 bytes.
function imageType(b: Uint8Array): ImageType | null {
  if (b.length < 12) return null;
  const starts = (sig: number[]) => sig.every((v, i) => b[i] === v);
  if (starts([0xff, 0xd8, 0xff])) return "jpeg";
  if (starts([0x89, 0x50, 0x4e, 0x47])) return "png";
  if (starts([0x47, 0x49, 0x46, 0x38])) return "gif";
  if (starts([0x52, 0x49, 0x46, 0x46]) && b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42) {
    return "webp";
  }
  return "other";
}

const EOF = "failed to fill whole buffer";

/// The width and height in an image's header, like `imagesize::blob_size`, for PNG, JPEG
/// and GIF; `null` for a format it can't read.
function blobSize(b: Uint8Array): Result<[number, number], string> | null {
  const t = imageType(b);
  if (t === null) return err(EOF);
  const u32 = (at: number) =>
    at + 4 > b.length ? null : ((b[at]! << 24) | (b[at + 1]! << 16) | (b[at + 2]! << 8) | b[at + 3]!) >>> 0;
  const u16be = (at: number) => (at + 2 > b.length ? null : (b[at]! << 8) | b[at + 1]!);
  const u16le = (at: number) => (at + 2 > b.length ? null : b[at]! | (b[at + 1]! << 8));
  switch (t) {
    case "png": {
      const w = u32(0x10);
      const h = u32(0x14);
      return w === null || h === null ? err(EOF) : ok([w, h]);
    }
    case "gif": {
      const w = u16le(6);
      const h = u16le(8);
      return w === null || h === null ? err(EOF) : ok([w, h]);
    }
    case "jpeg": {
      let pos = 2;
      let depth = 0;
      for (;;) {
        if (pos + 2 > b.length) return err(EOF);
        const [m0, page] = [b[pos]!, b[pos + 1]!];
        pos += 2;
        if (m0 !== 0xff) return err("Hit end of file before finding size");
        if (
          (page >= 0xc0 && page <= 0xc3) ||
          (page >= 0xc5 && page <= 0xc7) ||
          (page >= 0xc9 && page <= 0xcb) ||
          (page >= 0xcd && page <= 0xcf)
        ) {
          if (depth === 0) {
            pos += 3;
            break;
          }
        } else if (page === 0xd8) {
          depth += 1;
        } else if (page === 0xd9) {
          depth -= 1;
          if (depth < 0) return err("Hit end of file before finding size");
        }
        const size = u16be(pos);
        if (size === null) return err(EOF);
        pos += size;
        if (pos < 0) return err("invalid seek to a negative or overflowing position");
      }
      const h = u16be(pos);
      const w = u16be(pos + 2);
      return w === null || h === null ? err(EOF) : ok([w, h]);
    }
    case "webp":
      return null;
    case "other":
      return err("Could not decode image");
  }
}

function utf8(bytes: Uint8Array): string | null {
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    return null;
  }
}

/// A color as `#rrggbb` and its alpha (0..255), as usvg resolves a `#rgb`, `#rrggbb` or
/// `#rrggbbaa` value; other values are returned unchanged with full alpha.
function splitColor(c: string): [string, number] {
  const m = /^#([0-9a-fA-F]+)$/.exec(c);
  if (m === null) return [c, 255];
  let h = m[1]!.toLowerCase();
  if (h.length === 3 || h.length === 4) h = [...h].map((x) => x + x).join("");
  if (h.length === 6) return [`#${h}`, 255];
  if (h.length === 8) return [`#${h.slice(0, 6)}`, parseInt(h.slice(6), 16)];
  return [c, 255];
}

/// The shortest decimal that reads back as the same `f32`.
function f32Str(v: number): string {
  const f = Math.fround(v);
  for (let p = 1; p <= 9; p++) {
    const n = Number(f.toPrecision(p));
    if (Math.fround(n) === f) return String(n);
  }
  return String(f);
}

/// A bundled icon from its precomputed usvg output, with `currentColor` in `color`.
function bundledSvg(path: string, name: string, color: string | null): Asset | null {
  const icon = bundledIcon(name);
  if (icon === null) return null;
  const currentColor = color !== null && usesCurrentColor(new TextEncoder().encode(icon.source)) ? color : null;
  const [hex, alpha] = splitColor(currentColor ?? "#000000");
  let svgText = icon.svg;
  if (alpha < 255) {
    svgText = svgText
      .split(`stroke="${ICON_SENTINEL_COLOR}"`)
      .join(`stroke="${hex}" stroke-opacity="${f32Str(alpha / 255)}"`)
      .split(`fill="${ICON_SENTINEL_COLOR}"`)
      .join(`fill="${hex}" fill-opacity="${f32Str(alpha / 255)}"`);
  }
  svgText = svgText.split(ICON_SENTINEL_COLOR).join(hex);
  return new Asset({
    path,
    mime: "image/svg+xml",
    natural: [icon.natural[0], icon.natural[1]],
    bytes: new TextEncoder().encode(svgText),
    unitHull: icon.unit_hull.map(([x, y]) => new Pt(x, y)),
    currentColor,
    dropped: icon.dropped.map(([n, c]) => [n, c]),
  });
}

/// Decodes and sanitizes an SVG. This is not usvg: a bundled icon uses its precomputed usvg
/// output, and any other file is parsed, stripped of scripts, event handlers,
/// `foreignObject` and external references, its shapes and text written as paths and the
/// rest written back as it was, so the `out.svg` of render goldens that embed an SVG file
/// differ from the Rust output.
function svg(path: string, bytes: Uint8Array, color: string | null): Result<Asset, string> {
  const depth = checkDepth(bytes);
  if (!depth.ok) return depth;
  if (path.startsWith("builtin:")) {
    const a = bundledSvg(path, path.slice("builtin:".length), color);
    if (a !== null) return ok(a);
  }
  const currentColor = color !== null && usesCurrentColor(bytes) ? color : null;
  const text = utf8(bytes);
  if (text === null) return err("not a valid SVG: provided data has not an UTF-8 encoding");
  const parsed = parseXml(text);
  if (!parsed.ok) return err(`not a valid SVG: ${parsed.error}`);
  const doc = parsed.value;
  if (localName(doc.root.name) !== "svg") {
    return err("not a valid SVG: the root element is not `svg`");
  }
  const natural = svgSize(doc.root);
  if (natural === null) return err("not a valid SVG: SVG has an invalid size");
  const source = descendants(doc.root);
  const state: Sanitize = { tooDeep: false, nested: null };
  sanitize(doc.root, state, new Map());
  if (state.tooDeep) {
    return err(`an embedded SVG nests elements more than ${MAX_SVG_DEPTH} deep`);
  }
  if (state.nested !== null) return err(state.nested);
  resolveCurrentColor(doc.root, currentColor);
  const [w, h] = natural;
  const pts = svgPoints(doc.root, natural);
  const unit = pts.map((p) => new Pt(p.x / w, p.y / h));
  const hull = convexHull(unit);
  return ok(
    new Asset({
      path,
      mime: "image/svg+xml",
      natural,
      bytes: new TextEncoder().encode(serializeXml(doc)),
      unitHull: hull.length >= 3 ? hull : unitSquare(),
      currentColor,
      dropped: dropped(source, doc.root),
    }),
  );
}

/// The elements of the SVG source that sanitizing lost, as (element name, count):
/// `<image>`s that reference anything but a `data:` URI or fail to decode, and
/// `foreignObject`s.
function dropped(source: readonly XmlElement[], tree: XmlElement): [string, number][] {
  const count = (els: readonly XmlElement[], name: string) =>
    els.filter((e) => localName(e.name) === name).length;
  const out: [string, number][] = [
    ["image", Math.max(count(source, "image") - count(descendants(tree), "image"), 0)],
    ["foreignObject", count(source, "foreignObject")],
  ];
  return out.filter(([, n]) => n > 0);
}

function localName(name: string): string {
  const i = name.indexOf(":");
  return i < 0 ? name : name.slice(i + 1);
}

const UNITS: Readonly<Record<string, number>> = {
  "": 1,
  px: 1,
  in: 96,
  cm: 96 / 2.54,
  mm: 96 / 25.4,
  pt: 4 / 3,
  pc: 16,
};

function parseLength(s: string | null): [number, string] | null {
  if (s === null) return null;
  const m = /^\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)\s*(px|in|cm|mm|pt|pc|%)?\s*$/.exec(s);
  return m === null ? null : [Number(m[1]), m[2] ?? ""];
}

function viewBox(el: XmlElement): [number, number, number, number] | null {
  const v = attr(el, "viewBox");
  if (v === null) return null;
  const n = v
    .trim()
    .split(/[\s,]+/)
    .map(Number);
  if (n.length !== 4 || n.some((x) => !Number.isFinite(x)) || n[2]! <= 0 || n[3]! <= 0) {
    return null;
  }
  return [n[0]!, n[1]!, n[2]!, n[3]!];
}

/// The size usvg gives the root `svg`: `width` and `height` (100% when missing), with
/// percentages taken of the `viewBox`, or of 100×100 without one.
function svgSize(root: XmlElement): [number, number] | null {
  const vb = viewBox(root);
  const axis = (name: string, full: number | null): number => {
    const l = parseLength(attr(root, name)) ?? [100, "%"];
    if (l[1] === "%") return ((full ?? 100) * l[0]) / 100;
    return l[0] * UNITS[l[1]]!;
  };
  const w = Math.fround(axis("width", vb === null ? null : vb[2]));
  const h = Math.fround(axis("height", vb === null ? null : vb[3]));
  if (!(w > 0) || !(h > 0) || !Number.isFinite(w) || !Number.isFinite(h)) return null;
  return [w, h];
}

interface Sanitize {
  tooDeep: boolean;
  nested: string | null;
}

const STRIPPED = new Set(["script", "foreignObject"]);
const BASIC_SHAPES = new Set(["rect", "circle", "ellipse", "line", "polyline", "polygon"]);
const SHAPE_ATTRS = new Set([
  "x",
  "y",
  "width",
  "height",
  "rx",
  "ry",
  "cx",
  "cy",
  "r",
  "x1",
  "y1",
  "x2",
  "y2",
  "points",
]);
const TEXT_ATTRS = new Set([
  "x",
  "y",
  "dx",
  "dy",
  "rotate",
  "textLength",
  "lengthAdjust",
  "font-size",
  "font-family",
  "font-weight",
  "font-style",
  "text-anchor",
]);
const FONT_ATTRS = ["font-size", "font-family", "font-weight", "text-anchor"];
/// Families with a bundled face: the named fonts and the generic families.
const FAMILIES = [
  "inter",
  "jetbrains mono",
  "monospace",
  "sans-serif",
  "serif",
  "cursive",
  "fantasy",
];
/// Paths written for `<text>`, which `svgPoints` leaves out as usvg's hull does.
const FROM_TEXT = new WeakSet<XmlElement>();

/// Removes scripts, `foreignObject`s, event handlers, external references and images that
/// don't decode, checks embedded images as `check_nested_images` does, and writes basic
/// shapes and text as paths, text in the bundled fonts.
function sanitize(el: XmlElement, state: Sanitize, inherited: ReadonlyMap<string, string>): void {
  el.attrs = el.attrs.filter((a) => {
    const n = a.name.toLowerCase();
    if (n.startsWith("on")) return false;
    if (localName(a.name) === "href") {
      const v = a.value.trim();
      return v.startsWith("data:") || v.startsWith("#");
    }
    return true;
  });
  const own = new Map(inherited);
  for (const k of FONT_ATTRS) {
    const v = attr(el, k);
    if (v !== null) own.set(k, v);
  }
  const children: XmlNode[] = [];
  for (const c of el.children) {
    if (c.kind !== "element") {
      children.push(c);
      continue;
    }
    const name = localName(c.name);
    if (STRIPPED.has(name)) continue;
    if (name === "image" && !keepImage(c, state)) continue;
    if (BASIC_SHAPES.has(name)) {
      const d = shapeD(c);
      if (d === null) continue;
      c.name = "path";
      c.attrs = c.attrs.filter((a) => !SHAPE_ATTRS.has(a.name)).concat([{ name: "d", value: d }]);
    }
    if (name === "text") {
      const t = textPath(c, own);
      if (t !== null) children.push(t);
      continue;
    }
    sanitize(c, state, own);
    children.push(c);
  }
  el.children = children;
}

/// A `<text>` as one path of its glyph outlines in the bundled fonts: Inter, or JetBrains
/// Mono for the first known family that is `monospace` or `JetBrains Mono`.
function textPath(el: XmlElement, inherited: ReadonlyMap<string, string>): XmlElement | null {
  const get = (k: string) => attr(el, k) ?? inherited.get(k) ?? null;
  const content = textContent(el).replace(/\s+/g, " ").trim();
  if (content === "") return null;
  const first = (k: string) => {
    const v = attr(el, k);
    const n = v === null ? NaN : Number(v.trim().split(/[\s,]+/)[0]);
    return Number.isFinite(n) ? n : 0;
  };
  const sizeL = parseLength(get("font-size"));
  const size = sizeL === null || sizeL[1] === "%" ? 12 : sizeL[0] * UNITS[sizeL[1]]!;
  const weight = get("font-weight")?.trim() ?? "normal";
  const bold = weight === "bold" || weight === "bolder" || Number(weight) >= 600;
  const family =
    (get("font-family") ?? "")
      .split(",")
      .map((f) => f.trim().replace(/^['"]|['"]$/g, "").toLowerCase())
      .find((f) => FAMILIES.includes(f)) ?? "";
  const mono = family === "monospace" || family === "jetbrains mono";
  const run = new Run(content, mono ? "mono" : bold ? "bold" : "regular");
  const width = runWidth(run, size);
  const anchor = get("text-anchor")?.trim();
  const x = first("x") - (anchor === "middle" ? width / 2 : anchor === "end" ? width : 0);
  const d = outline([run], size, x, first("y"));
  if (d === "") return null;
  const p: XmlElement = {
    kind: "element",
    name: "path",
    attrs: el.attrs.filter((a) => !TEXT_ATTRS.has(a.name)).concat([{ name: "d", value: d }]),
    children: [],
  };
  FROM_TEXT.add(p);
  return p;
}

function textContent(el: XmlElement): string {
  let s = "";
  for (const c of el.children) {
    if (c.kind === "text" || c.kind === "cdata") s += c.text;
    else if (c.kind === "element") s += textContent(c);
  }
  return s;
}

function href(el: XmlElement): string | null {
  return el.attrs.find((a) => localName(a.name) === "href")?.value ?? null;
}

function keepImage(el: XmlElement, state: Sanitize): boolean {
  const h = href(el)?.trim() ?? null;
  if (h === null || !h.startsWith("data:")) return false;
  const data = dataUrl(h);
  if (data === null) return false;
  const [mime, bytes] = data;
  let kind: "svg" | "raster" | null;
  switch (mime) {
    case "image/jpg":
    case "image/jpeg":
    case "image/png":
    case "image/gif":
    case "image/webp":
      kind = "raster";
      break;
    case "image/svg+xml":
      kind = "svg";
      break;
    case "text/plain": {
      const t = imageType(bytes);
      kind = t === "jpeg" || t === "png" || t === "gif" || t === "webp" ? "raster" : "svg";
      break;
    }
    default:
      kind = null;
  }
  if (kind === null) return false;
  if (kind === "raster") {
    const size = blobSize(bytes);
    if (size !== null && state.nested === null) {
      if (!size.ok) {
        state.nested = `cannot read an embedded image's size: ${size.error}`;
      } else {
        const c = checkImageSize(size.value[0], size.value[1]);
        if (!c.ok) state.nested = `an embedded image: ${c.error}`;
      }
    }
    return true;
  }
  if (!checkDepth(bytes).ok) {
    state.tooDeep = true;
    return false;
  }
  const text = utf8(bytes);
  const sub = text === null ? null : parseXml(text);
  if (sub === null || !sub.ok || localName(sub.value.root.name) !== "svg") return false;
  sanitize(sub.value.root, state, new Map());
  return true;
}

/// The media type and bytes of a `data:` URI.
function dataUrl(url: string): [string, Uint8Array] | null {
  const comma = url.indexOf(",");
  if (comma < 0) return null;
  const meta = url.slice("data:".length, comma);
  const parts = meta.split(";").map((p) => p.trim());
  const base64 = parts.slice(1).some((p) => p.toLowerCase() === "base64");
  const mime = parts[0] === "" ? "text/plain" : parts[0]!.toLowerCase();
  const body = url.slice(comma + 1);
  if (base64) {
    const clean = body.replace(/\s+/g, "");
    if (!/^[A-Za-z0-9+/]*={0,2}$/.test(clean)) return null;
    return [mime, new Uint8Array(Buffer.from(clean, "base64"))];
  }
  const out: number[] = [];
  const enc = new TextEncoder();
  for (let i = 0; i < body.length; i++) {
    const c = body[i]!;
    if (c === "%" && /^[0-9a-fA-F]{2}$/.test(body.slice(i + 1, i + 3))) {
      out.push(parseInt(body.slice(i + 1, i + 3), 16));
      i += 2;
    } else {
      out.push(...enc.encode(c));
    }
  }
  return [mime, new Uint8Array(out)];
}

const PAINTS: readonly [string, string][] = [
  ["fill", "fill-opacity"],
  ["stroke", "stroke-opacity"],
  ["stop-color", "stop-opacity"],
  ["flood-color", "flood-opacity"],
  ["lighting-color", ""],
];

/// Replaces `currentColor` with the nearest `color=` above it, else `color` (the shape's),
/// else black.
function resolveCurrentColor(root: XmlElement, color: string | null): void {
  const walk = (el: XmlElement, inherited: [string, number]) => {
    const own = attr(el, "color");
    const cur: [string, number] = own !== null ? [own, 255] : inherited;
    const extra: { name: string; value: string }[] = [];
    for (const a of el.attrs) {
      const paint = PAINTS.find(([p]) => p === a.name);
      if (paint !== undefined && a.value.trim().toLowerCase() === "currentcolor") {
        a.value = cur[0];
        if (cur[1] < 255 && paint[1] !== "" && attr(el, paint[1]) === null) {
          extra.push({ name: paint[1], value: f32Str(cur[1] / 255) });
        }
      } else if (a.name === "style") {
        a.value = a.value.replace(/currentcolor/gi, cur[0]);
      }
    }
    el.attrs.push(...extra);
    for (const c of el.children) if (c.kind === "element") walk(c, cur);
  };
  walk(root, splitColor(color ?? "#000000"));
}

const NOT_DRAWN = new Set([
  "defs",
  "clipPath",
  "mask",
  "pattern",
  "symbol",
  "marker",
  "linearGradient",
  "radialGradient",
  "filter",
  "style",
  "title",
  "desc",
  "metadata",
]);

/// The path data of a basic shape (`rect`, `circle`, `ellipse`, `line`, `polyline`,
/// `polygon`) as usvg builds it, or of a `path`; `null` for other elements and shapes
/// with no area to draw.
function shapeD(el: XmlElement): string | null {
  const num = (name: string) => {
    const l = parseLength(attr(el, name));
    return l === null || l[1] === "%" ? 0 : l[0] * UNITS[l[1]]!;
  };
  const name = localName(el.name);
  switch (name) {
    case "path":
      return attr(el, "d");
    case "rect": {
      const [x, y, w, h] = [num("x"), num("y"), num("width"), num("height")];
      if (!(w > 0 && h > 0)) return null;
      let rx = attr(el, "rx") !== null ? num("rx") : null;
      let ry = attr(el, "ry") !== null ? num("ry") : null;
      rx ??= ry ?? 0;
      ry ??= rx;
      rx = Math.min(rx, w / 2);
      ry = Math.min(ry, h / 2);
      return rx > 0 && ry > 0
        ? `M ${x + rx} ${y} L ${x + w - rx} ${y} A ${rx} ${ry} 0 0 1 ${x + w} ${y + ry} L ${x + w} ${y + h - ry} A ${rx} ${ry} 0 0 1 ${x + w - rx} ${y + h} L ${x + rx} ${y + h} A ${rx} ${ry} 0 0 1 ${x} ${y + h - ry} L ${x} ${y + ry} A ${rx} ${ry} 0 0 1 ${x + rx} ${y} Z`
        : `M ${x} ${y} L ${x + w} ${y} L ${x + w} ${y + h} L ${x} ${y + h} Z`;
    }
    case "circle":
    case "ellipse": {
      const [cx, cy] = [num("cx"), num("cy")];
      const rx = name === "circle" ? num("r") : num("rx");
      const ry = name === "circle" ? rx : num("ry");
      if (!(rx > 0 && ry > 0)) return null;
      return `M ${cx + rx} ${cy} A ${rx} ${ry} 0 0 1 ${cx} ${cy + ry} A ${rx} ${ry} 0 0 1 ${cx - rx} ${cy} A ${rx} ${ry} 0 0 1 ${cx} ${cy - ry} A ${rx} ${ry} 0 0 1 ${cx + rx} ${cy} Z`;
    }
    case "line":
      return `M ${num("x1")} ${num("y1")} L ${num("x2")} ${num("y2")}`;
    case "polyline":
    case "polygon": {
      const p = attr(el, "points");
      if (p === null) return null;
      return name === "polygon" ? `M ${p} Z` : `M ${p}`;
    }
    default:
      return null;
  }
}

/// Every point of the drawn paths, control points included, in the SVG's canvas
/// coordinates: the `viewBox` is applied, element transforms are not. Text is left out.
function svgPoints(root: XmlElement, natural: [number, number]): Pt[] {
  const out: Pt[] = [];
  const walk = (el: XmlElement) => {
    if (NOT_DRAWN.has(localName(el.name)) || FROM_TEXT.has(el)) return;
    const d = shapeD(el);
    if (d !== null) out.push(...pathPoints(d));
    for (const c of el.children) if (c.kind === "element") walk(c);
  };
  walk(root);
  const vb = viewBox(root);
  if (vb === null) return out;
  const [w, h] = natural;
  const [vx, vy, vw, vh] = vb;
  const par = attr(root, "preserveAspectRatio")?.trim() ?? "";
  let [sx, sy] = [w / vw, h / vh];
  if (!par.startsWith("none")) {
    const s = par.includes("slice") ? Math.max(sx, sy) : Math.min(sx, sy);
    sx = s;
    sy = s;
  }
  const tx = -vx * sx + (w - vw * sx) / 2;
  const ty = -vy * sy + (h - vh * sy) / 2;
  return out.map((p) => new Pt(p.x * sx + tx, p.y * sy + ty));
}

/// The points of an SVG path's data, control points included, with arcs as cubics.
/// Parsing stops at the first error, as SVG renderers do.
export function pathPoints(d: string): Pt[] {
  const out: Pt[] = [];
  const toks: (string | number)[] = [];
  const re = /([MmLlHhVvCcSsQqTtAaZz])|([+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)|([\s,]+)|(.)/g;
  let m: RegExpExecArray | null;
  let lastCmd = "";
  let argIndex = 0;
  while ((m = re.exec(d)) !== null) {
    if (m[1] !== undefined) {
      toks.push(m[1]);
      lastCmd = m[1].toLowerCase();
      argIndex = 0;
    } else if (m[2] !== undefined) {
      const isFlag = lastCmd === "a" && (argIndex % 7 === 3 || argIndex % 7 === 4);
      if (isFlag && /^[01]/.test(m[2]) && m[2].length > 1) {
        toks.push(Number(m[2][0]));
        re.lastIndex = m.index + 1;
      } else {
        toks.push(Number(m[2]));
      }
      argIndex++;
    } else if (m[4] !== undefined) {
      break;
    }
  }
  let i = 0;
  let cur = new Pt(0, 0);
  let start = cur;
  let cmd = "";
  let lastCtrl: Pt | null = null;
  let lastKind = "";
  const nums = (n: number): number[] | null => {
    const v: number[] = [];
    for (let k = 0; k < n; k++) {
      const t = toks[i + k];
      if (typeof t !== "number") return null;
      v.push(t);
    }
    i += n;
    return v;
  };
  const push = (p: Pt) => out.push(p);
  while (i < toks.length) {
    const t = toks[i];
    if (typeof t === "string") {
      cmd = t;
      i++;
      if (cmd === "Z" || cmd === "z") {
        cur = start;
        lastKind = "z";
        continue;
      }
    } else if (cmd === "" || cmd === "Z" || cmd === "z") {
      break;
    }
    const rel = cmd === cmd.toLowerCase();
    const at = (x: number, y: number) => (rel ? new Pt(cur.x + x, cur.y + y) : new Pt(x, y));
    const c = cmd.toUpperCase();
    let v: number[] | null;
    switch (c) {
      case "M":
        if ((v = nums(2)) === null) return out;
        cur = at(v[0]!, v[1]!);
        start = cur;
        push(cur);
        cmd = rel ? "l" : "L";
        lastKind = "m";
        break;
      case "L":
        if ((v = nums(2)) === null) return out;
        cur = at(v[0]!, v[1]!);
        push(cur);
        lastKind = "l";
        break;
      case "H":
        if ((v = nums(1)) === null) return out;
        cur = new Pt(rel ? cur.x + v[0]! : v[0]!, cur.y);
        push(cur);
        lastKind = "l";
        break;
      case "V":
        if ((v = nums(1)) === null) return out;
        cur = new Pt(cur.x, rel ? cur.y + v[0]! : v[0]!);
        push(cur);
        lastKind = "l";
        break;
      case "C": {
        if ((v = nums(6)) === null) return out;
        const [c1, c2, to] = [at(v[0]!, v[1]!), at(v[2]!, v[3]!), at(v[4]!, v[5]!)];
        push(c1);
        push(c2);
        push(to);
        lastCtrl = c2;
        cur = to;
        lastKind = "c";
        break;
      }
      case "S": {
        if ((v = nums(4)) === null) return out;
        const c1 =
          lastKind === "c" && lastCtrl !== null ? cur.add(cur.sub(lastCtrl)) : cur;
        const [c2, to] = [at(v[0]!, v[1]!), at(v[2]!, v[3]!)];
        push(c1);
        push(c2);
        push(to);
        lastCtrl = c2;
        cur = to;
        lastKind = "c";
        break;
      }
      case "Q": {
        if ((v = nums(4)) === null) return out;
        const [ctrl, to] = [at(v[0]!, v[1]!), at(v[2]!, v[3]!)];
        push(ctrl);
        push(to);
        lastCtrl = ctrl;
        cur = to;
        lastKind = "q";
        break;
      }
      case "T": {
        if ((v = nums(2)) === null) return out;
        const ctrl: Pt = lastKind === "q" && lastCtrl !== null ? cur.add(cur.sub(lastCtrl)) : cur;
        const to = at(v[0]!, v[1]!);
        push(ctrl);
        push(to);
        lastCtrl = ctrl;
        cur = to;
        lastKind = "q";
        break;
      }
      case "A": {
        if ((v = nums(7)) === null) return out;
        const to = at(v[5]!, v[6]!);
        const segs: PathSeg[] = arcToCubics(cur, v[0]!, v[1]!, v[2]!, v[3] !== 0, v[4] !== 0, to);
        for (const s of segs) {
          if (s.kind === "cubic") {
            push(s.c1);
            push(s.c2);
          }
          push(s.to);
        }
        cur = to;
        lastKind = "a";
        break;
      }
      default:
        return out;
    }
  }
  return out;
}

/// Whether an SVG paints with `currentColor` and leaves its root's `color` unset.
export function usesCurrentColor(bytes: Uint8Array): boolean {
  const text = utf8(bytes);
  if (text === null) {
    return false;
  }
  if (!text.toLowerCase().includes("currentcolor")) {
    return false;
  }
  const d = parseXml(text);
  return d.ok && attr(d.value.root, "color") === null;
}

/// Refuses markup nested more than `MAX_SVG_DEPTH` deep before the recursive parsers see
/// it. Comments, CDATA and processing instructions are skipped; declarations (`<!...>`)
/// are scanned through, so element markup in entity values counts too.
export function checkDepth(bytes: Uint8Array): Result<null, string> {
  const startsWith = (at: number, pat: string) => {
    for (let k = 0; k < pat.length; k++) {
      if (bytes[at + k] !== pat.charCodeAt(k)) return false;
    }
    return true;
  };
  const after = (from: number, pat: string) => {
    for (let i = from; i + pat.length <= bytes.length; i++) {
      if (startsWith(i, pat)) return i + pat.length;
    }
    return bytes.length;
  };
  let depth = 0;
  let i = 0;
  while (i < bytes.length) {
    if (bytes[i] !== 0x3c) {
      i += 1;
      continue;
    }
    if (startsWith(i, "<!--")) {
      i = after(i + 4, "-->");
    } else if (startsWith(i, "<![CDATA[")) {
      i = after(i + 9, "]]>");
    } else if (startsWith(i, "<?")) {
      i = after(i + 2, "?>");
    } else if (startsWith(i, "<!")) {
      i += 2;
    } else if (startsWith(i, "</")) {
      depth = Math.max(depth - 1, 0);
      i = after(i + 2, ">");
    } else {
      let j = i + 1;
      let quote: number | null = null;
      while (j < bytes.length) {
        const c = bytes[j]!;
        if (quote === null && (c === 0x22 || c === 0x27)) {
          quote = c;
        } else if (quote !== null && c === quote) {
          quote = null;
        } else if (quote === null && c === 0x3e) {
          break;
        }
        j += 1;
      }
      if (bytes[j - 1] !== 0x2f) {
        depth += 1;
        if (depth > MAX_SVG_DEPTH) {
          return err(`elements are nested more than ${MAX_SVG_DEPTH} deep`);
        }
      }
      i = j + 1;
    }
  }
  return ok(null);
}
