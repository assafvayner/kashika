import { Float, type Json } from "../diag/json.js";
import { type Size } from "../syntax/index.js";
import { type VAlign } from "../text/index.js";
import { type Align, type TitlePos } from "./scene.js";

export const STYLE_KEYS: readonly string[] = [
  "fill",
  "stroke",
  "stroke-width",
  "stroke-style",
  "font-size",
  "font-weight",
  "color",
  "radius",
  "padding",
  "muted",
  "text-align",
  "text-valign",
  "title",
];

/// Keys a `style` definition takes: the style keys, `shape` and `size`.
export function defKeys(): string[] {
  return [...STYLE_KEYS, "shape", "size"];
}

/// A built-in style: colors only, so it sets no shape or size.
export class BuiltinStyle {
  constructor(
    readonly name: string,
    readonly fill: string,
    readonly stroke: string,
    readonly strokeStyle: StrokeStyle | null,
    /// The text color, when the style sets one.
    readonly color: string | null,
  ) {}

  static colors(name: string, fill: string, stroke: string): BuiltinStyle {
    return new BuiltinStyle(name, fill, stroke, null, null);
  }

  props(): StyleProps {
    const p = StyleProps.default();
    p.fill = this.fill;
    p.stroke = this.stroke;
    p.strokeStyle = this.strokeStyle;
    p.color = this.color;
    return p;
  }
}

/// Every built-in style, in the order docs list them.
export const STYLES: readonly BuiltinStyle[] = [
  BuiltinStyle.colors("service", "#e8f0fe", "#1a73e8"),
  BuiltinStyle.colors("datastore", "#e6f4ea", "#137333"),
  BuiltinStyle.colors("external", "#f1f3f4", "#5f6368"),
  BuiltinStyle.colors("queue", "#fef7e0", "#b06000"),
  new BuiltinStyle("boundary", "none", "#94a3b8", "dashed", "#475569"),
  BuiltinStyle.colors("note", "#fffbe6", "#d4b106"),
];

export type StrokeStyle = "solid" | "dashed" | "dotted";

export const StrokeStyle = {
  NAMES: ["solid", "dashed", "dotted"] as readonly string[],

  fromName(name: string): StrokeStyle | null {
    return (
      (["solid", "dashed", "dotted"] as const).find((x) => StrokeStyle.name(x) === name) ?? null
    );
  },

  name(s: StrokeStyle): string {
    return s;
  },
};

export type FontWeight = "normal" | "bold";

/// The text area inset: `x` on the left and right, `y` on the top and bottom.
export class Padding {
  constructor(
    readonly x: number,
    readonly y: number,
  ) {}

  static uniform(v: number): Padding {
    return new Padding(v, v);
  }

  eq(o: Padding): boolean {
    return this.x === o.x && this.y === o.y;
  }

  /// One number when both axes match, otherwise `[x, y]`.
  toJson(): Json {
    if (this.x === this.y) {
      return new Float(this.x);
    } else {
      return [new Float(this.x), new Float(this.y)];
    }
  }
}

/// What `currentColor` in an icon resolves to when neither the shape nor a style it names sets
/// `color` (a group's built-in `boundary` color doesn't count): the default stroke, which
/// bundled icons were drawn in.
export const ICON_COLOR = "#334155";

export class Style {
  constructor(
    public fill: string,
    public stroke: string,
    public strokeWidth: number,
    public strokeStyle: StrokeStyle,
    public fontSize: number,
    public fontWeight: FontWeight,
    public color: string,
    public radius: number,
    public padding: Padding,
    public muted: string,
  ) {}

  static default(): Style {
    return new Style(
      "#ffffff",
      "#334155",
      1.5,
      "solid",
      14.0,
      "normal",
      "#0f172a",
      0.0,
      Padding.uniform(8.0),
      "#64748b",
    );
  }

  clone(): Style {
    return new Style(
      this.fill,
      this.stroke,
      this.strokeWidth,
      this.strokeStyle,
      this.fontSize,
      this.fontWeight,
      this.color,
      this.radius,
      this.padding,
      this.muted,
    );
  }

  eq(o: Style): boolean {
    return (
      this.fill === o.fill &&
      this.stroke === o.stroke &&
      this.strokeWidth === o.strokeWidth &&
      this.strokeStyle === o.strokeStyle &&
      this.fontSize === o.fontSize &&
      this.fontWeight === o.fontWeight &&
      this.color === o.color &&
      this.radius === o.radius &&
      this.padding.eq(o.padding) &&
      this.muted === o.muted
    );
  }

  toJson(): Json {
    return {
      fill: this.fill,
      stroke: this.stroke,
      stroke_width: new Float(this.strokeWidth),
      stroke_style: this.strokeStyle,
      font_size: new Float(this.fontSize),
      font_weight: this.fontWeight,
      color: this.color,
      radius: new Float(this.radius),
      padding: this.padding.toJson(),
      muted: this.muted,
    };
  }
}

/// A partial style; `null` means "inherit".
export class StyleProps {
  fill: string | null = null;
  stroke: string | null = null;
  strokeWidth: number | null = null;
  strokeStyle: StrokeStyle | null = null;
  fontSize: number | null = null;
  fontWeight: FontWeight | null = null;
  color: string | null = null;
  radius: number | null = null;
  padding: Padding | null = null;
  muted: string | null = null;
  textAlign: Align | null = null;
  /// Boxes only.
  textValign: VAlign | null = null;
  /// Groups only.
  title: TitlePos | null = null;
  /// The qualified shape name; boxes only.
  shape: string | null = null;
  /// Boxes and groups only.
  size: Size | null = null;

  static default(): StyleProps {
    return new StyleProps();
  }

  clone(): StyleProps {
    return Object.assign(new StyleProps(), this);
  }

  eq(o: StyleProps): boolean {
    const optEq = <T>(a: T | null, b: T | null, f: (a: T, b: T) => boolean) =>
      a === null || b === null ? a === b : f(a, b);
    return (
      this.fill === o.fill &&
      this.stroke === o.stroke &&
      this.strokeWidth === o.strokeWidth &&
      this.strokeStyle === o.strokeStyle &&
      this.fontSize === o.fontSize &&
      this.fontWeight === o.fontWeight &&
      this.color === o.color &&
      this.radius === o.radius &&
      optEq(this.padding, o.padding, (a, b) => a.eq(b)) &&
      this.muted === o.muted &&
      this.textAlign === o.textAlign &&
      this.textValign === o.textValign &&
      this.title === o.title &&
      this.shape === o.shape &&
      optEq(this.size, o.size, (a, b) => a.w === b.w && a.h === b.h)
    );
  }

  /// Returns `this` with every key that `top` sets replaced by `top`'s value.
  overlay(top: StyleProps): StyleProps {
    const p = new StyleProps();
    p.fill = top.fill ?? this.fill;
    p.stroke = top.stroke ?? this.stroke;
    p.strokeWidth = top.strokeWidth ?? this.strokeWidth;
    p.strokeStyle = top.strokeStyle ?? this.strokeStyle;
    p.fontSize = top.fontSize ?? this.fontSize;
    p.fontWeight = top.fontWeight ?? this.fontWeight;
    p.color = top.color ?? this.color;
    p.radius = top.radius ?? this.radius;
    p.padding = top.padding ?? this.padding;
    p.muted = top.muted ?? this.muted;
    p.textAlign = top.textAlign ?? this.textAlign;
    p.textValign = top.textValign ?? this.textValign;
    p.title = top.title ?? this.title;
    p.shape = top.shape ?? this.shape;
    p.size = top.size ?? this.size;
    return p;
  }

  resolve(): Style {
    const d = Style.default();
    return new Style(
      this.fill ?? d.fill,
      this.stroke ?? d.stroke,
      this.strokeWidth ?? d.strokeWidth,
      this.strokeStyle ?? d.strokeStyle,
      this.fontSize ?? d.fontSize,
      this.fontWeight ?? d.fontWeight,
      this.color ?? d.color,
      this.radius ?? d.radius,
      this.padding ?? d.padding,
      this.muted ?? d.muted,
    );
  }

  /// Only the keys that apply to lines.
  lineKeys(): StyleProps {
    const p = new StyleProps();
    p.stroke = this.stroke;
    p.strokeWidth = this.strokeWidth;
    p.strokeStyle = this.strokeStyle;
    p.fontSize = this.fontSize;
    p.fontWeight = this.fontWeight;
    p.color = this.color;
    return p;
  }
}

export function builtin(name: string): StyleProps | null {
  return STYLES.find((s) => s.name === name)?.props() ?? null;
}
