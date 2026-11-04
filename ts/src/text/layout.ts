import { Float, type Json } from "../diag/json.js";
import { type Result, fmax, fmin, ok, sum } from "../diag/rust.js";
import { Rect } from "../geom/index.js";
import { LINE_HEIGHT, baselineOffset, missingGlyphs, runWidth } from "./font.js";
import { type MarkupError, type Run, parseMarkup } from "./markup.js";

export type HAlign = "left" | "center" | "right";

export type VAlign = "top" | "middle" | "bottom";

export interface MeasuredLine {
  runs: Run[];
  width: number;
}

/// A line positioned on the canvas. The pen starts at `(x, baseline)`.
export class PlacedLine {
  constructor(
    public runs: Run[],
    public x: number,
    public baseline: number,
    public width: number,
  ) {}

  toJson(): Json {
    return {
      runs: this.runs.map((r) => r.toJson()),
      x: new Float(this.x),
      baseline: new Float(this.baseline),
      width: new Float(this.width),
    };
  }
}

export function measure(text: string, size: number, bold: boolean): Result<Block, MarkupError> {
  const parsed = parseMarkup(text, bold);
  if (!parsed.ok) return parsed;
  const lines: MeasuredLine[] = parsed.value.map((runs) => {
    const width = sum(runs.map((r) => runWidth(r, size)));
    return { runs, width };
  });
  const width = lines.map((l) => l.width).reduce(fmax, 0);
  const height = lines.length * size * LINE_HEIGHT;
  return ok(new Block(lines, size, width, height));
}

/// One measured paragraph: a string split on `\n`, with markup applied, at one size.
export class Block {
  readonly #lines: MeasuredLine[];
  readonly #size: number;
  readonly #width: number;
  readonly #height: number;

  constructor(lines: MeasuredLine[], size: number, width: number, height: number) {
    this.#lines = lines;
    this.#size = size;
    this.#width = width;
    this.#height = height;
  }

  lines(): readonly MeasuredLine[] {
    return this.#lines;
  }

  size(): number {
    return this.#size;
  }

  width(): number {
    return this.#width;
  }

  height(): number {
    return this.#height;
  }

  /// The distinct characters, in order of first appearance, that the bundled fonts have
  /// no glyph for. They are drawn as the font's `.notdef` shape.
  missingGlyphs(): string[] {
    return missingGlyphs(this.#lines.flatMap((l) => l.runs));
  }

  /// Places the lines top-down from `top`, each aligned within `[left, left + width]`.
  /// Returns the lines and their bounding box.
  place(left: number, width: number, top: number, align: HAlign): [PlacedLine[], Rect] {
    const lh = this.#size * LINE_HEIGHT;
    const base = baselineOffset(this.#size);
    const lines = this.#lines.map(
      (l, i) =>
        new PlacedLine(
          l.runs.map((r) => r.clone()),
          align === "left"
            ? left
            : align === "center"
              ? left + (width - l.width) / 2
              : left + width - l.width,
          top + lh * i + base,
          l.width,
        ),
    );
    const x0 = lines.map((l) => l.x).reduce(fmin, Infinity);
    const x1 = lines.map((l) => l.x + l.width).reduce(fmax, -Infinity);
    return [lines, new Rect(x0, top, x1 - x0, this.#height)];
  }

  /// Places the block with `at` as its horizontal alignment point and vertical center.
  placeAt(x: number, y: number, align: HAlign): [PlacedLine[], Rect] {
    const left = align === "left" ? x : align === "center" ? x - this.#width / 2 : x - this.#width;
    return this.place(left, this.#width, y - this.#height / 2, align);
  }
}

/// Width and height of blocks stacked top to bottom with `gap` between them.
export function stackSize(blocks: readonly Block[], gap: number): [number, number] {
  const w = blocks.map((b) => b.width()).reduce(fmax, 0);
  const h = sum(blocks.map((b) => b.height())) + gap * Math.max(blocks.length - 1, 0);
  return [w, h];
}

/// Stacks blocks inside `area`. Each block is aligned horizontally by its own `HAlign`;
/// the stack as a whole is aligned vertically by `valign`.
export function stack(
  items: readonly (readonly [Block, HAlign])[],
  area: Rect,
  valign: VAlign,
  gap: number,
): [PlacedLine[], Rect][] {
  const blocks = items.map(([b]) => b);
  const [, h] = stackSize(blocks, gap);
  let top =
    valign === "top" ? area.y : valign === "middle" ? area.y + (area.h - h) / 2 : area.bottom() - h;
  const out: [PlacedLine[], Rect][] = [];
  for (const [b, align] of items) {
    out.push(b.place(area.x, area.w, top, align));
    top += b.height() + gap;
  }
  return out;
}
