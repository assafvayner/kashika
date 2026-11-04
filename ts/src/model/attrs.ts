import { Diagnostic } from "../diag/diagnostic.js";
import { didYouMean } from "../diag/distance.js";
import { cmpTuple, displayF64 } from "../diag/rust.js";
import { type Attr, type Size } from "../syntax/index.js";
import { type VAlign } from "../text/index.js";
import * as colors from "./colors.js";
import { Align, TitlePos } from "./scene.js";
import { type FontWeight, Padding, StrokeStyle, StyleProps } from "./style.js";

/// Attribute names written where another name is meant: (written, meant). Each is suggested
/// only where the meant name is allowed and the written one isn't.
const MEANT: readonly (readonly [string, string])[] = [
  ["size", "font-size"],
  ["text-size", "font-size"],
  ["align", "text-align"],
  ["valign", "text-valign"],
  ["vertical-align", "text-valign"],
  ["font-color", "color"],
  ["text-color", "color"],
  ["weight", "font-weight"],
];

/// Validated view over a statement's attributes: reports unknown keys (with suggestions),
/// duplicates, and `@t` on keys other than `label`.
export class Attrs {
  private readonly list: Attr[];

  constructor(list: readonly Attr[], what: string, allowed: readonly string[], diags: Diagnostic[]) {
    const seen: [string, number | null][] = [];
    const kept: Attr[] = [];
    for (const a of list) {
      const key = a.key.name;
      if (!allowed.includes(key)) {
        const d = Diagnostic.error("attr", a.key.span, `unknown attribute \`${key}\` on ${what}`);
        const h = unknownHelp(a, what, allowed);
        if (h !== null) {
          d.withHelp(h);
        }
        if (what === "style") {
          const keys = allowed.map((k) => `\`${k}\``);
          d.withHelp(`style keys: ${keys.join(", ")}`);
        }
        diags.push(d);
        continue;
      }
      if (a.at_t !== null && key !== "label") {
        diags.push(
          Diagnostic.error("attr", a.span, `\`@\` positions only apply to \`label\`, not \`${key}\``),
        );
        continue;
      }
      const k: [string, number | null] = [key, a.at_t];
      if (seen.some(([sk, st]) => sk === k[0] && Object.is(st, k[1]))) {
        diags.push(Diagnostic.error("attr", a.key.span, `duplicate attribute \`${key}\``));
        continue;
      }
      seen.push(k);
      kept.push(a);
    }
    this.list = kept;
  }

  get(key: string): Attr | null {
    return this.list.find((a) => a.key.name === key && a.at_t === null) ?? null;
  }

  all(key: string): Attr[] {
    return this.list.filter((a) => a.key.name === key);
  }
}

/// The help for an attribute `allowed` doesn't take: `size (w,h)` for a `size=` pair, else the
/// name meant in this context, else the nearest allowed name.
function unknownHelp(a: Attr, what: string, allowed: readonly string[]): string | null {
  const key = a.key.name;
  if (key === "size" && (a.value.kind === "Pair" || a.value.kind === "Size")) {
    return "`size` is a clause, not an attribute: write `size (w,h)`, without `=`";
  }
  if (key === "label" && (what === "box" || what === "group" || what === "text")) {
    const text = a.value.kind === "Str" ? a.value.value : "…";
    const id = what === "text" ? "" : "<id> ";
    return `the label is the quoted string after the id, with no \`label=\`: \`${what} ${id}"${text}" …\``;
  }
  if ((key === "arrow" || key === "arrowhead" || key === "head") && allowed.includes("end")) {
    const ends =
      a.value.kind === "Word" && a.value.value === "start"
        ? "`start=arrow`"
        : a.value.kind === "Word" && a.value.value === "both"
          ? "`start=arrow end=arrow`"
          : "`end=arrow`";
    return `arrowheads are line ends: write ${ends}, or use the \`arrow\` statement`;
  }
  if (
    (key === "width" || key === "height" || key === "w" || key === "h") &&
    (what === "box" || what === "group" || what === "image")
  ) {
    return "the size is a clause: write `size (w,h)`, without `=`";
  }
  if (key === "title" && what === "box") {
    return "`title=` places a group's title; a box places its text with `text-valign`";
  }
  if (StrokeStyle.NAMES.includes(key) && allowed.includes("stroke-style")) {
    return `write \`stroke-style=${key}\``;
  }
  const s =
    MEANT.find(([written, meant]) => written === key && allowed.includes(meant))?.[1] ??
    didYouMean(key, allowed);
  return s === null ? null : `did you mean \`${s}\`?`;
}

/// The attribute set as `name` or as its `alias`. Setting both is a duplicate, reported at
/// the later one; the earlier one is kept.
export function aliased(a: Attrs, name: string, alias: string, diags: Diagnostic[]): Attr | null {
  const x = a.get(name);
  const y = a.get(alias);
  if (x !== null && y !== null) {
    const [first, second] =
      cmpTuple([y.span.line, y.span.col], [x.span.line, x.span.col]) < 0 ? [y, x] : [x, y];
    diags.push(
      Diagnostic.error(
        "attr",
        second.key.span,
        `duplicate attribute: \`${alias}\` is another name for \`${name}\``,
      ).withHelp(`keep one of \`${first.key.name}\` and \`${second.key.name}\``),
    );
    return first;
  }
  return x ?? y;
}

export function typeErr(a: Attr, expected: string): Diagnostic {
  const d = Diagnostic.error("attr", a.span, `\`${a.key.name}\` expects ${expected}`);
  if (a.value.kind === "Word" && expected === "a string") {
    return d.withHelp(`quote it: \`${a.key.name}="${a.value.value}"\``);
  }
  return d;
}

export function num(a: Attr, diags: Diagnostic[]): number | null {
  if (a.value.kind === "Num") {
    return a.value.value;
  } else {
    diags.push(typeErr(a, "a number"));
    return null;
  }
}

/// Largest magnitude allowed for any coordinate, size or numeric attribute.
export const LIMIT = 1e6;

export type Min = "Positive" | "NonNegative";

/// A number in `min..=LIMIT`; anything else is an `attr` error.
export function numIn(a: Attr, min: Min, diags: Diagnostic[]): number | null {
  const v = num(a, diags);
  if (v === null) return null;
  const low =
    min === "Positive" && v <= 0.0
      ? "greater than 0"
      : min === "NonNegative" && v < 0.0
        ? "at least 0"
        : null;
  let msg: string;
  if (low !== null) {
    msg = `\`${a.key.name}\` must be ${low}`;
  } else if (v > LIMIT) {
    msg = `\`${a.key.name}\` must be at most ${displayF64(LIMIT)}`;
  } else {
    return v;
  }
  diags.push(Diagnostic.error("attr", a.span, msg));
  return null;
}

/// `padding=N` or `padding=(x,y)`, each value in `0..=LIMIT`.
function padding(a: Attr, diags: Diagnostic[]): Padding | null {
  switch (a.value.kind) {
    case "Num": {
      const v = numIn(a, "NonNegative", diags);
      return v === null ? null : Padding.uniform(v);
    }
    case "Pair": {
      const [x, y] = a.value.value;
      const bad = (v: number) => !(v >= 0.0 && v <= LIMIT);
      if (bad(x) || bad(y)) {
        diags.push(
          Diagnostic.error(
            "attr",
            a.span,
            `\`padding\` values must be at least 0 and at most ${displayF64(LIMIT)}`,
          ).withHelp("write `padding=(x,y)` with x and y between 0 and 1000000"),
        );
        return null;
      }
      return new Padding(x, y);
    }
    default:
      diags.push(
        typeErr(a, "a number or a pair `(x,y)`").withHelp(
          "write `padding=8` for every side, or `padding=(x,y)` for left/right and top/bottom",
        ),
      );
      return null;
  }
}

/// A style's `size=(w,h)`: each axis a number in `0 < v <= LIMIT`, or `auto`.
export function size(a: Attr, diags: Diagnostic[]): Size | null {
  switch (a.value.kind) {
    case "Size": {
      const s = a.value.value;
      const bad = (v: number | null) => v !== null && !(v > 0.0 && v <= LIMIT);
      if (bad(s.w) || bad(s.h)) {
        diags.push(
          Diagnostic.error("size", a.span, `\`size\` must be positive and at most ${displayF64(LIMIT)}`),
        );
        return null;
      }
      return s;
    }
    case "Num":
      diags.push(typeErr(a, "a size `(w,h)`").withHelp("for the text size, write `font-size=N`"));
      return null;
    default:
      diags.push(typeErr(a, "a size `(w,h)`"));
      return null;
  }
}

export function text(a: Attr, diags: Diagnostic[]): string | null {
  if (a.value.kind === "Str") {
    return a.value.value;
  } else {
    diags.push(typeErr(a, "a string"));
    return null;
  }
}

export function color(a: Attr, diags: Diagnostic[]): string | null {
  const v = a.value;
  if (v.kind === "Color") {
    return v.value;
  }
  if (v.kind === "Word" && v.value === "none") {
    return v.value;
  }
  if (v.kind === "Word") {
    const hex = colors.hex(v.value);
    if (hex !== null) {
      return hex;
    }
    const d = typeErr(a, "a color like `#1a73e8`, a CSS color name or `none`");
    const s = colors.suggest(v.value);
    if (s !== null) {
      d.withHelp(`did you mean \`${s}\`?`);
    }
    diags.push(d);
    return null;
  }
  diags.push(typeErr(a, "a color like `#1a73e8`, a CSS color name or `none`"));
  return null;
}

export function flag(a: Attr, diags: Diagnostic[]): boolean {
  if (a.value.kind === "Flag") {
    return true;
  } else {
    diags.push(Diagnostic.error("attr", a.span, `\`${a.key.name}\` takes no value`));
    return false;
  }
}

export function word<T extends string>(
  a: Attr,
  options: readonly T[],
  diags: Diagnostic[],
): T | null {
  const v = a.value;
  if (v.kind === "Word") {
    const o = options.find((o) => o === v.value);
    if (o !== undefined) {
      return o;
    }
  }
  const d = Diagnostic.error("attr", a.span, `\`${a.key.name}\` expects one of: ${options.join(", ")}`);
  if (v.kind === "Word") {
    const s = suggestWord(v.value, options);
    if (s !== null) {
      d.withHelp(`did you mean \`${s}\`?`);
    }
  }
  diags.push(d);
  return null;
}

/// Words written where one of `options` is meant: (written, meant), checked before spelling.
const WORDS_MEANT: readonly (readonly [string, string])[] = [
  ["middle", "center"],
  ["mid", "center"],
  ["centre", "center"],
  ["north", "top"],
  ["south", "bottom"],
  ["east", "right"],
  ["west", "left"],
  ["arrowhead", "arrow"],
];

/// The option meant by `w`: a known synonym, else the nearest spelling.
export function suggestWord<T extends string>(w: string, options: readonly T[]): T | null {
  const m = WORDS_MEANT.find(([written, meant]) => written === w && options.includes(meant as T));
  const meant = m === undefined ? undefined : options.find((o) => o === m[1]);
  return meant ?? (didYouMean(w, options) as T | null);
}

export function styleProps(a: Attrs, diags: Diagnostic[]): StyleProps {
  const p = StyleProps.default();
  let x: Attr | null;
  if ((x = a.get("fill")) !== null) {
    p.fill = color(x, diags);
  }
  if ((x = a.get("stroke")) !== null) {
    p.stroke = color(x, diags);
  }
  if ((x = a.get("stroke-width")) !== null) {
    p.strokeWidth = numIn(x, "Positive", diags);
  }
  if ((x = a.get("stroke-style")) !== null) {
    const w = word(x, StrokeStyle.NAMES, diags);
    p.strokeStyle = w === null ? null : StrokeStyle.fromName(w);
  }
  if ((x = a.get("font-size")) !== null) {
    p.fontSize = numIn(x, "Positive", diags);
  }
  if ((x = a.get("font-weight")) !== null) {
    const w = word(x, ["normal", "bold"], diags);
    p.fontWeight = w === null ? null : ((w === "bold" ? "bold" : "normal") satisfies FontWeight);
  }
  if ((x = a.get("color")) !== null) {
    p.color = color(x, diags);
  }
  if ((x = a.get("radius")) !== null) {
    p.radius = numIn(x, "NonNegative", diags);
  }
  if ((x = a.get("padding")) !== null) {
    p.padding = padding(x, diags);
  }
  if ((x = a.get("muted")) !== null) {
    p.muted = color(x, diags);
  }
  if ((x = a.get("text-align")) !== null) {
    const w = word(x, Align.NAMES, diags);
    p.textAlign = w === null ? null : Align.fromName(w);
  }
  if ((x = a.get("text-valign")) !== null) {
    p.textValign = valign(x, diags);
  }
  if ((x = a.get("title")) !== null) {
    const w = word(x, TitlePos.NAMES, diags);
    p.title = w === null ? null : TitlePos.fromName(w);
  }
  return p;
}

export function valign(a: Attr, diags: Diagnostic[]): VAlign | null {
  const w = word(a, ["top", "middle", "bottom"], diags);
  if (w === null) return null;
  switch (w) {
    case "top":
      return "top";
    case "bottom":
      return "bottom";
    default:
      return "middle";
  }
}
