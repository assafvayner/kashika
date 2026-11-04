import { Diagnostic, type Span } from "../../diag/diagnostic.js";
import { didYouMean } from "../../diag/distance.js";
import { displayF64 } from "../../diag/rust.js";
import { Pt, Rect } from "../../geom/index.js";
import type * as ast from "../../syntax/ast.js";
import * as anchor from "../anchor.js";
import * as attrs from "../attrs.js";
import { Attrs } from "../attrs.js";
import { Form } from "../form.js";
import * as library from "../library.js";
import { LibShape, type SvgShape } from "../library.js";
import * as markup from "../markup.js";
import {
  IMAGE_KEYS,
  type IconPos,
  type IconProps,
  JUNCTION_KEYS,
  LINE_KEYS,
  type LineProps,
  type NodeProps,
  type Props,
  type Resolver,
  TEXT_KEYS,
  type TextProps,
  nodeKeys,
} from "../resolve.js";
import {
  type Align as AlignT,
  Align,
  type Allow,
  Fit,
  Label,
  LabelSide,
  type Layer,
  Marker,
} from "../scene.js";
import * as shapes from "../shapes.js";
import * as style from "../style.js";
import { StrokeStyle, Style, StyleProps } from "../style.js";
import * as textlayout from "../textlayout.js";
import { ShapeTextProps } from "../textlayout.js";
import { allowList, sizeOk } from "./checks.js";
import { pin } from "./placement.js";

/// How far the icon pushes the text area from the left and from the top edge.
export function nodeIconInset(p: NodeProps, group: boolean): [number, number] {
  const ic = p.icon;
  if (ic === null) return [0.0, 0.0];
  const pad = p.style.padding;
  if (group || ic.pos === "Left") {
    return [ic.size + pad.x, 0.0];
  } else {
    return [0.0, ic.size + pad.y];
  }
}

/// The shape drawn: the `shape=` name, else `rounded` or `rect` from the outline.
export function drawnShape(p: NodeProps): string {
  if (p.shape !== null) return p.shape;
  return p.outline === "rounded" ? "rounded" : "rect";
}

/// Reads every entry's attributes into its props, loading the files they name.
/// Skipped entries (duplicate ids and their children) get none.
export function validate(r: Resolver): void {
  for (let idx = 0; idx < r.entries.length; idx++) {
    if (r.skipped.has(idx)) {
      continue;
    }
    const item = r.entries[idx]!.item;
    let props: Props;
    switch (item.kind) {
      case "Box":
        props = { kind: "Node", value: nodeProps(r, item.value, false) };
        break;
      case "Group":
        props = { kind: "Node", value: nodeProps(r, item.value.node, true) };
        break;
      case "Image":
        props = { kind: "Node", value: imageProps(r, item.value) };
        break;
      case "Junction": {
        const a = new Attrs(item.value.attrs, "junction", JUNCTION_KEYS, r.diags);
        const x = a.get("dot");
        const dot = x !== null && attrs.flag(x, r.diags);
        props = { kind: "Junction", dot };
        break;
      }
      case "Text":
        props = { kind: "Text", value: textProps(r, item.value) };
        break;
      case "Line":
        props = { kind: "Line", value: lineProps(r, item.value) };
        break;
    }
    r.entries[idx]!.props = props;
  }
}

/// `allow=` and `reason=`, shared by boxes, groups, images and lines.
function allowReason(r: Resolver, a: Attrs): [Allow[], string | null] {
  const ax = a.get("allow");
  const allow = ax === null ? [] : allowList(r, ax, true);
  const rx = a.get("reason");
  const t = rx === null ? null : attrs.text(rx, r.diags);
  const reason = t === null ? null : markup.unescape(t);
  return [allow, reason];
}

export function nodeProps(r: Resolver, n: ast.Node, group: boolean): NodeProps {
  const what = group ? "group" : "box";
  const a = new Attrs(n.attrs, what, nodeKeys(group), r.diags);

  const sx = a.get("shape");
  const shapeIn = sx === null ? null : shapeAttr(r, sx);
  let props = StyleProps.default();
  let color: string | null = null;
  if (group) {
    const user = r.userStyles.get("boundary")?.clone() ?? StyleProps.default();
    color = user.color;
    props = props.overlay(style.builtin("boundary") ?? StyleProps.default()).overlay(user);
  }
  const stx = a.get("style");
  if (stx !== null) {
    const named = namedStyle(r, stx);
    if (named !== null) {
      if (group && named.shape !== null) {
        r.diags.push(
          Diagnostic.error(
            "attr",
            stx.span,
            "this style sets `shape`, but groups take no shape",
          ).withHelp("use a style without `shape`, or drop `shape` from it"),
        );
      }
      if (group && named.textValign !== null) {
        r.diags.push(
          Diagnostic.error(
            "attr",
            stx.span,
            "this style sets `text-valign`, but a group's title sits at its top or bottom edge",
          ).withHelp(
            "use a style without `text-valign`, or drop `text-valign` from it; `title=bottom` puts the title at the bottom",
          ),
        );
      }
      if (!group && named.title !== null) {
        r.diags.push(
          Diagnostic.error(
            "attr",
            stx.span,
            "this style sets `title`, but only groups have a title",
          ).withHelp(
            "use a style without `title`, or drop `title` from it; a box places its text with `text-valign`",
          ),
        );
      }
      color = named.color ?? color;
      props = props.overlay(named);
    }
  }
  const own = attrs.styleProps(a, r.diags);
  color = own.color ?? color;
  props = props.overlay(own);
  const shape = group ? null : (shapeIn ?? props.shape);
  const rounded = shape === "rounded";
  if (rounded && props.radius === null) {
    props.radius = 8.0;
  }
  let outline: NodeProps["outline"];
  if (shape !== null && shape !== "rect" && shape !== "rounded") {
    outline = "path";
  } else if (rounded || (props.radius ?? 0.0) > 0.0) {
    outline = "rounded";
  } else {
    outline = "rect";
  }
  const px = a.get("pin");
  const pinV = (px === null ? null : pin(r, px)) ?? ["tl", null];
  const text = textlayout.shapeTextProps(a, props, group, r.diags);
  const [allow, reason] = allowReason(r, a);
  const st = props.resolve();
  const iconColor = color !== null && color !== "none" ? color : style.ICON_COLOR;
  const icon = iconProps(r, a, group, st.fontSize, iconColor);
  const layer = layerOf(r, a);
  const p: NodeProps = {
    outline,
    shape,
    style: st,
    pin: pinV,
    size: null,
    fitted: [false, false],
    text,
    allow,
    reason,
    icon,
    image: null,
    layer,
  };
  p.size = nodeSize(r, n, group, p, props.size);
  if (group && p.size !== null) {
    const { w, h } = n.size ?? props.size ?? { w: null, h: null };
    p.fitted = [w === null, h === null];
    if (px !== null) {
      if (!pinStays(p.pin, p.fitted)) {
        const [pname, t] = p.pin;
        const shown = t === null ? pname : `${pname}@${displayF64(t)}`;
        r.diags.push(
          Diagnostic.error(
            "attr",
            px.span,
            `\`pin=${shown}\` moves as \`${n.id.name}\` grows to fit its children`,
          ).withHelp(
            "a fitted group grows right and down from its top-left corner: pin it at `tl`, on its left edge for a fitted width or its top edge for a fitted height, or give it a size: `size (w,h)`",
          ),
        );
        p.pin = ["tl", null];
      }
    }
  }
  return p;
}

/// The size of a box or group: its `size`, else its style's, with each `auto` axis (or
/// both, when neither gives one) fitted to its text and rounded up to the `grid`.
function nodeSize(
  r: Resolver,
  n: ast.Node,
  group: boolean,
  p: NodeProps,
  styled: ast.Size | null,
): [number, number] | null {
  const what = `\`${n.id.name}\``;
  const { w, h } = n.size ?? styled ?? { w: null, h: null };
  if (w !== null && h !== null) {
    return sizeOk(r, [w, h], n.span, what) ? [w, h] : null;
  }
  if (!sizeOk(r, [w ?? 1.0, h ?? 1.0], n.span, what)) {
    return null;
  }
  if (group) {
    return [w ?? 0.0, h ?? 0.0];
  }
  const shape = drawnShape(p);
  const lib = r.lib.shapes.get(shape);
  const svg = svgShape(r, shape);
  const input: textlayout.ShapeInput = {
    id: n.id.name,
    node: n,
    kind: "box",
    shape,
    customTextArea: lib === undefined ? null : LibShape.textArea(lib),
    textBelow: svg !== null && svg.below,
    contain: contained(r, shape),
    iconInset: nodeIconInset(p, false),
    rect: new Rect(0.0, 0.0, w ?? 0.0, h ?? 0.0),
    style: p.style,
    props: p.text,
    blockAt: [],
  };
  const fit = textlayout.fitSize(input);
  if (fit === null) {
    const why = input.textBelow
      ? "its text sits under it (`text-area=below`)"
      : "it has no text";
    r.diags.push(
      Diagnostic.error("size", n.span, `${what} has no size to fit: ${why}`).withHelp(
        "give it a size: `size (w,h)`",
      ),
    );
    return null;
  }
  const grid = r.grid;
  const snap = (v: number) => (grid === null ? v : Math.ceil(v / grid) * grid);
  const size: [number, number] = [w ?? snap(fit[0]), h ?? snap(fit[1])];
  return sizeOk(r, size, n.span, what) ? size : null;
}

function layerOf(r: Resolver, a: Attrs): Layer {
  const x = a.get("layer");
  const w = x === null ? null : attrs.word(x, ["under", "over"], r.diags);
  switch (w) {
    case "under":
      return "under";
    case "over":
      return "over";
    default:
      return "normal";
  }
}

export function iconProps(
  r: Resolver,
  a: Attrs,
  group: boolean,
  fontSize: number,
  color: string,
): IconProps | null {
  let pos: IconPos;
  const posX = a.get("icon-pos");
  if (posX !== null) {
    const w = attrs.word(posX, ["left", "top"], r.diags);
    if (w === "top" && group) {
      r.diags.push(Diagnostic.error("attr", posX.span, "groups only support `icon-pos=left`"));
      pos = "Left";
    } else if (w === "top") {
      pos = "Top";
    } else {
      pos = "Left";
    }
  } else {
    pos = "Left";
  }
  const sizeX = a.get("icon-size");
  const size =
    (sizeX === null ? null : attrs.numIn(sizeX, "Positive", r.diags)) ??
    (group ? fontSize * 1.25 : 24.0);
  const x = a.get("icon");
  if (x === null) {
    for (const k of ["icon-pos", "icon-size"]) {
      const y = a.get(k);
      if (y !== null) {
        r.diags.push(Diagnostic.error("attr", y.span, `\`${k}\` needs \`icon=\``));
      }
    }
    return null;
  }
  const name = attrs.text(x, r.diags);
  if (name === null) return null;
  const root = r.loader.root();
  const res = r.store.loadIcon(r.loader, root, name, color);
  if (!res.ok) {
    r.fileError(x.span, res.error);
    return null;
  }
  return { asset: res.value, pos, size };
}

export function imageProps(r: Resolver, i: ast.Image): NodeProps {
  const a = new Attrs(i.attrs, "image", IMAGE_KEYS, r.diags);
  const size = sizeOk(r, i.size, i.span, `\`${i.id.name}\``) ? i.size : null;
  const px = a.get("pin");
  const pinV = (px === null ? null : pin(r, px)) ?? ["tl", null];
  const fx = a.get("fit");
  const fw = fx === null ? null : attrs.word(fx, Fit.NAMES, r.diags);
  const fit = (fw === null ? null : Fit.fromName(fw)) ?? "contain";
  const [allow, reason] = allowReason(r, a);
  const layer = layerOf(r, a);
  const asset = loadAsset(r, i.file, i.file_span);
  return {
    outline: "rect",
    shape: null,
    style: Style.default(),
    pin: pinV,
    size: size === null ? null : [size[0], size[1]],
    fitted: [false, false],
    text: ShapeTextProps.default(),
    allow,
    reason,
    icon: null,
    image: { asset, fit },
    layer,
  };
}

export function loadAsset(r: Resolver, name: string, span: Span): number | null {
  const root = r.loader.root();
  const res = r.store.load(r.loader, root, name);
  if (res.ok) {
    return res.value;
  }
  r.fileError(span, res.error);
  return null;
}

/// The shape a `shape=` attribute names, if it exists.
export function shapeAttr(r: Resolver, a: ast.Attr): string | null {
  return library.shapeName(a, "", r.lib.shapes, r.diags);
}

/// The outline and drawing of a shape name on `rect`: a custom definition, else a
/// built-in sized for `rect`. `null` for `rect` and `rounded`.
export function formOf(r: Resolver, name: string, rect: Rect): Form | null {
  const s = r.lib.shapes.get(name);
  if (s !== undefined) {
    if (s.kind === "unit") return Form.fromUnit(s.value, rect);
    return Form.fromSvg(r.store.get(s.value.asset), s.value.hull, s.value.fit, rect);
  }
  const u = shapes.builtin(name, rect.w, rect.h);
  return u === null ? null : Form.fromUnit(u, rect);
}

export function svgShape(r: Resolver, name: string): SvgShape | null {
  const s = r.lib.shapes.get(name);
  return s !== undefined && s.kind === "svg" ? s.value : null;
}

/// The natural size of an SVG-file shape drawn with `fit=contain`.
export function contained(r: Resolver, name: string): [number, number] | null {
  const s = svgShape(r, name);
  if (s === null || s.fit !== "contain") return null;
  const n = r.store.get(s.asset).natural;
  return [n[0], n[1]];
}

export function namedStyle(r: Resolver, a: ast.Attr): StyleProps | null {
  if (a.value.kind !== "Word") {
    r.diags.push(attrs.typeErr(a, "a style name"));
    return null;
  }
  const name = a.value.value;
  const builtin = style.builtin(name);
  const user = r.userStyles.get(name)?.clone() ?? null;
  if (builtin === null && user === null) {
    const names: string[] = style.STYLES.map((s) => s.name).concat([...r.userStyles.keys()]);
    const d = Diagnostic.error("unknown-style", a.span, `unknown style \`${name}\``);
    if (StrokeStyle.NAMES.includes(name)) {
      d.withHelp(`a stroke pattern is \`stroke-style=${name}\``);
    } else {
      const s = didYouMean(name, names);
      if (s !== null) {
        d.withHelp(`did you mean \`${s}\`?`);
      }
    }
    r.diags.push(d);
    return null;
  }
  return (builtin ?? StyleProps.default()).overlay(user ?? StyleProps.default());
}

export function textProps(r: Resolver, t: ast.Text): TextProps {
  const a = new Attrs(t.attrs, "text", TEXT_KEYS, r.diags);
  const sx = attrs.aliased(a, "font-size", "size", r.diags);
  const size = (sx === null ? null : textlayout.textSize(sx, r.diags)) ?? 14.0;
  const mx = a.get("muted");
  const muted = mx !== null && attrs.flag(mx, r.diags);
  const cx = a.get("color");
  let color = cx === null ? null : textlayout.textColor(cx, r.diags);
  if (color === null) {
    const d = Style.default();
    color = muted ? d.muted : d.color;
  }
  const ax = attrs.aliased(a, "text-align", "align", r.diags);
  const aw = ax === null ? null : attrs.word(ax, Align.NAMES, r.diags);
  const align: AlignT = (aw === null ? null : Align.fromName(aw)) ?? "center";
  const content = t.content;
  return { content, size, color, align };
}

export function lineProps(r: Resolver, l: ast.Line): LineProps {
  const a = new Attrs(l.attrs, "line", LINE_KEYS, r.diags);
  const marker = (key: string): Marker => {
    const x = a.get(key);
    const w = x === null ? null : attrs.word(x, Marker.NAMES, r.diags);
    return (w === null ? null : Marker.fromName(w)) ?? "none";
  };
  const start = marker("start");
  let end: Marker;
  if (a.get("end") !== null) {
    end = marker("end");
  } else if (l.arrow) {
    end = "arrow";
  } else {
    end = "none";
  }
  let props = StyleProps.default();
  props.fontSize = 12.0;
  const stx = a.get("style");
  if (stx !== null) {
    const named = namedStyle(r, stx);
    if (named !== null) {
      props = props.overlay(named.lineKeys());
    }
  }
  props = props.overlay(attrs.styleProps(a, r.diags));
  const labels: Label[] = [];
  for (const x of a.all("label")) {
    const t = x.at_t ?? 0.5;
    if (!(t >= 0.0 && t <= 1.0)) {
      r.diags.push(
        Diagnostic.error("attr", x.span, `label position ${displayF64(t)} is outside 0..1`),
      );
      continue;
    }
    if (labels.some((lab) => lab.t === t)) {
      r.diags.push(Diagnostic.error("attr", x.span, `duplicate label at t=${displayF64(t)}`));
      continue;
    }
    const text = attrs.text(x, r.diags);
    if (text !== null) {
      labels.push(
        new Label({
          t,
          text,
          at: Pt.default(),
          rect: Rect.default(),
          lines: [],
          side: "on",
          span: x.span,
        }),
      );
    }
  }
  const sideAttr = a.get("label-side");
  const sw = sideAttr === null ? null : attrs.word(sideAttr, LabelSide.NAMES, r.diags);
  const labelSide = (sw === null ? null : LabelSide.fromName(sw)) ?? LabelSide.default();
  const ox = a.get("ortho");
  let ortho: boolean;
  if (ox === null) {
    ortho = r.ortho;
  } else if (ox.value.kind === "Flag") {
    ortho = true;
  } else if (ox.value.kind === "Word" && ox.value.value === "false") {
    ortho = false;
  } else {
    r.diags.push(
      Diagnostic.error(
        "attr",
        ox.span,
        "`ortho` is a flag; the only value it takes is `false`",
      ).withHelp("write `ortho`, or `ortho=false` to opt out of the diagram's `ortho`"),
    );
    ortho = r.ortho;
  }
  const orthoInherited = ortho && ox === null;
  const [allow, reason] = allowReason(r, a);
  const radius = props.radius ?? 0.0;
  return {
    start,
    end,
    style: props.resolve(),
    labels,
    labelSide,
    labelSideSpan: sideAttr === null ? l.span : sideAttr.span,
    ortho,
    orthoInherited,
    allow,
    reason,
    radius,
  };
}

/// Whether a pin's anchor is on the left edge when the width is fitted, and on the top
/// edge when the height is, so the pinned point doesn't move as the group grows.
function pinStays([name, t]: [string, number | null], [fw, fh]: [boolean, boolean]): boolean {
  const unit = new Rect(0.0, 0.0, 1.0, 1.0);
  const u = anchor.resolve(unit, 0.0, "pin", name, t);
  return u.ok && (!fw || u.value.x === 0.0) && (!fh || u.value.y === 0.0);
}
