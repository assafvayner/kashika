import { Diagnostic } from "../../diag/diagnostic.js";
import { Pt, Rect } from "../../geom/index.js";
import * as ast from "../../syntax/ast.js";
import { type Axis, type Coord, type Point, type Target, offsetDelta } from "../../syntax/ast.js";
import * as anchor from "../anchor.js";
import * as attrs from "../attrs.js";
import * as form from "../form.js";
import { CYCLE_SHOWN, Entry, Item, MAX_PLACEMENT_DEPTH, type Resolver } from "../resolve.js";
import * as textlayout from "../textlayout.js";
import { ensureBuilt, name } from "./build.js";
import { Reach } from "./canvas.js";
import { finite, sizeOk, unknownId } from "./checks.js";
import { drawnShape, formOf, nodeIconInset } from "./props.js";
import * as record from "./record.js";

/// The entries `idx`'s position depends on: its group (or that group's frame, when the
/// group is fitted), then each id in its `at`. A fitted group's rect depends on its frame
/// and everything inside it; texts and lines only take part inside a fitted group.
function placementDeps(r: Resolver, idx: number, frameOf: ReadonlyMap<number, number>): number[] {
  const f = frameOf.get(idx);
  if (f !== undefined) {
    const out = [f];
    const [from, to] = descendants(r, idx);
    for (let d = from; d < to; d++) {
      if (!r.skipped.has(d)) out.push(d);
    }
    return out;
  }
  const item = r.entries[idx]!.item;
  let refs: ast.Ref[];
  switch (item.kind) {
    case "Box":
      refs = record.pointRefs(item.value.at);
      break;
    case "Image":
      refs = record.pointRefs(item.value.at);
      break;
    case "Junction":
      refs = record.pointRefs(item.value.at);
      break;
    case "Text":
      if (!inFitted(r, idx)) return [];
      refs = record.pointRefs(item.value.at);
      break;
    case "Line":
      if (!inFitted(r, idx)) return [];
      refs = record.linePoints(item.value).flatMap(record.pointRefs);
      break;
    case "Group":
      return frameDeps(r, idx, frameOf);
  }
  return withParent(r, idx, refs, frameOf);
}

/// What a group's frame depends on: its parent's frame and the ids in its `at`.
function frameDeps(r: Resolver, idx: number, frameOf: ReadonlyMap<number, number>): number[] {
  const item = r.entries[idx]!.item;
  if (item.kind !== "Group") {
    return [];
  }
  return withParent(r, idx, record.pointRefs(item.value.node.at), frameOf);
}

function withParent(
  r: Resolver,
  idx: number,
  refs: readonly ast.Ref[],
  frameOf: ReadonlyMap<number, number>,
): number[] {
  const placeable = (i: number) => {
    const k = r.entries[i]!.item.kind;
    return k !== "Text" && k !== "Line";
  };
  const p = r.entries[idx]!.parent;
  const out: number[] = [];
  if (p !== null) out.push(frameOf.get(p) ?? p);
  for (const ref of refs) {
    const i = r.ids.get(ref.id.name);
    if (i !== undefined && placeable(i)) out.push(i);
  }
  return out;
}

/// The entries inside group `idx`, at any depth, as a half-open range; they follow it in order.
export function descendants(r: Resolver, idx: number): [number, number] {
  const depth = r.entries[idx]!.depth;
  let end = r.entries.length;
  for (let i = idx + 1; i < r.entries.length; i++) {
    if (r.entries[i]!.depth <= depth) {
      end = i;
      break;
    }
  }
  return [idx + 1, end];
}

export function isFitted(r: Resolver, idx: number): boolean {
  const p = Entry.node(r.entries[idx]!);
  return p !== null && (p.fitted[0] || p.fitted[1]);
}

function inFitted(r: Resolver, idx: number): boolean {
  let p = r.entries[idx]!.parent;
  while (p !== null) {
    if (isFitted(r, p)) {
      return true;
    }
    p = r.entries[p]!.parent;
  }
  return false;
}

/// Reports placement cycles and over-long reference chains before anything is placed,
/// whatever the statement order, and marks the entries involved as unplaceable. A fitted
/// group is two nodes: its frame, which its children use, and its rect, which needs them.
export function placementGraph(r: Resolver): void {
  const n = r.entries.length;
  const fitted: number[] = [];
  for (let i = 0; i < n; i++) {
    if (!r.skipped.has(i) && isFitted(r, i)) fitted.push(i);
  }
  const frameOf = new Map<number, number>(fitted.map((g, k) => [g, n + k]));
  const deps: number[][] = [];
  for (let i = 0; i < n; i++) {
    deps.push(r.skipped.has(i) ? [] : placementDeps(r, i, frameOf));
  }
  for (const g of fitted) deps.push(frameDeps(r, g, frameOf));
  const entry = (i: number) => (i < n ? i : fitted[i - n]!);
  const total = deps.length;
  const sccs = tarjan(deps);
  const cyclic: boolean[] = new Array<boolean>(total).fill(false);
  for (const scc of sccs) {
    const looped = scc.length > 1 || deps[scc[0]!]!.includes(scc[0]!);
    if (!looped) {
      continue;
    }
    for (const i of scc) {
      cyclic[i] = true;
      unplaceable(r, i, n, fitted);
    }
    const start = Math.min(...scc);
    let cycle = shortestCycle(start, deps, scc);
    const inside = (g: number, d: number) => {
      const [from, to] = descendants(r, g);
      return d >= from && d < to;
    };
    const grown = cycle.findIndex(
      (i) => i < n && frameOf.has(i) && cycle.some((d) => d < n && inside(i, d)),
    );
    let help: string | null = null;
    if (grown >= 0) {
      const g = cycle[grown]!;
      const len = cycle.length;
      const k = (grown + 1) % len;
      cycle = cycle.slice(k).concat(cycle.slice(0, k));
      const gn = name(r, g);
      help = `\`${gn}\` is fitted to its children, so they can't be placed against its size; give \`${gn}\` a size: \`size (w,h)\``;
    }
    const named: number[] = [];
    for (const i of cycle) {
      const e = entry(i);
      if (named.length === 0 || named[named.length - 1] !== e) named.push(e);
    }
    if (named.length > 1 && named[0] === named[named.length - 1]) {
      named.pop();
    }
    reportCycle(r, named, help);
  }

  const depth: number[] = new Array<number>(total).fill(0);
  const next: (number | null)[] = new Array<number | null>(total).fill(null);
  for (const scc of sccs) {
    const i = scc[0]!;
    if (cyclic[i]) {
      continue;
    }
    for (const d of deps[i]!) {
      if (!cyclic[d] && depth[d]! + 1 > depth[i]!) {
        depth[i] = depth[d]! + 1;
        next[i] = d;
      }
    }
  }
  let head: number | null = null;
  for (let i = 0; i < total; i++) {
    if (depth[i]! > MAX_PLACEMENT_DEPTH && (head === null || depth[i]! > depth[head]!)) {
      head = i;
    }
  }
  if (head === null) {
    return;
  }
  let tail = head;
  while (next[tail] !== null) {
    tail = next[tail]!;
  }
  r.diags.push(
    Diagnostic.error(
      "placement-cycle",
      Item.atSpan(r.entries[entry(head)]!.item),
      `placement chain from \`${name(r, entry(head))}\` to \`${name(r, entry(tail))}\` is ${depth[head]} references deep; the limit is ${MAX_PLACEMENT_DEPTH}`,
    ).withHelp("place some shapes in this chain with literal coordinates"),
  );
  depth.forEach((d, i) => {
    if (d > MAX_PLACEMENT_DEPTH) {
      unplaceable(r, i, n, fitted);
    }
  });
}

function unplaceable(r: Resolver, node: number, n: number, fitted: readonly number[]): void {
  if (node < n) {
    r.placed[node] = null;
  } else {
    r.frames.set(fitted[node - n]!, null);
  }
}

export function reportCycle(r: Resolver, cycle: readonly number[], help: string | null): void {
  const names = cycle.map((i) => name(r, i));
  const first = names[0]!;
  const message =
    names.length <= CYCLE_SHOWN
      ? `placement cycle: ${names.join(" -> ")} -> ${first}`
      : `placement cycle of ${names.length} shapes: ${names.slice(0, 4).join(" -> ")} -> … -> ${names[names.length - 1]} -> ${first}`;
  r.diags.push(
    Diagnostic.error("placement-cycle", Item.atSpan(r.entries[cycle[0]!]!.item), message).withHelp(
      help ?? "place one of these with literal coordinates",
    ),
  );
}

/// Places an entry (box, group or junction), resolving dependencies depth-first.
export function place(r: Resolver, idx: number): Rect | null {
  const done = r.placed[idx];
  if (done !== undefined) {
    return done;
  }
  if (r.onStack.size > MAX_PLACEMENT_DEPTH || r.onStack.has(idx)) {
    return null;
  }
  r.onStack.add(idx);
  const rect = computePlace(r, idx);
  r.onStack.delete(idx);
  r.placed[idx] = rect;
  return rect;
}

export function pin(r: Resolver, a: ast.Attr): [string, number | null] | null {
  if (a.value.kind !== "AnchorAt") {
    const w = attrs.word(a, anchor.ANCHOR_NAMES, r.diags);
    return w === null ? null : [w, null];
  }
  const [nameW, t] = a.value.value;
  const found = anchor.ANCHOR_NAMES.find((n) => n === nameW);
  if (found === undefined) {
    const d = Diagnostic.error("attr", a.span, `unknown anchor \`${nameW}\` in \`pin\``);
    const s = attrs.suggestWord(nameW, anchor.ANCHOR_NAMES);
    if (s !== null) {
      d.withHelp(`did you mean \`${s}\`?`);
    }
    r.diags.push(d);
    return null;
  }
  const unit = new Rect(0.0, 0.0, 1.0, 1.0);
  const res = anchor.resolve(unit, 0.0, "pin", found, t);
  if (!res.ok) {
    const [msg, help] = res.error;
    const d = Diagnostic.error("unknown-anchor", a.span, `${msg} in \`pin\``);
    if (help !== null) {
      d.withHelp(help);
    }
    r.diags.push(d);
    return null;
  }
  return [found, t];
}

export function computePlace(r: Resolver, idx: number): Rect | null {
  const origin = frameOrigin(r, idx);
  if (origin === null) return null;
  const item = r.entries[idx]!.item;
  switch (item.kind) {
    case "Box":
      return placeNode(r, idx, item.value.at, origin);
    case "Group":
      return isFitted(r, idx)
        ? placeFitted(r, idx, item.value)
        : placeNode(r, idx, item.value.node.at, origin);
    case "Image":
      return placeNode(r, idx, item.value.at, origin);
    case "Junction": {
      const p = evalPoint(r, item.value.at, origin);
      return p === null ? null : new Rect(p.x, p.y, 0.0, 0.0);
    }
    case "Text":
    case "Line":
      return null;
  }
}

export function placeNode(r: Resolver, idx: number, at: Point, origin: Pt): Rect | null {
  const p0 = Entry.node(r.entries[idx]!);
  if (p0 === null || p0.size === null) return null;
  const [w, h] = p0.size;
  const atP = evalPoint(r, at, origin);
  if (atP === null) return null;

  const p = Entry.node(r.entries[idx]!);
  if (p === null) return null;
  const local = new Rect(0.0, 0.0, w, h);
  const [pinName, t] = p.pin;
  const radius = p.style.radius;
  const shape = drawnShape(p);
  const f0 = formOf(r, shape, local);
  const f = f0 === null ? null : new form.Form(f0.draw, f0.outline, null);
  const res = form.resolveAnchor(f, local, radius, "pin", pinName, t);
  const off = res.ok ? res.value : Pt.default();
  return new Rect(atP.x - off.x, atP.y - off.y, w, h);
}

/// Canvas position of the frame an entry's literal coordinates are written in.
export function frameOrigin(r: Resolver, idx: number): Pt | null {
  const p = r.entries[idx]!.parent;
  if (p === null) {
    return new Pt(0.0, 0.0);
  }
  if (isFitted(r, p)) {
    return fittedFrame(r, p);
  }
  const rect = place(r, p);
  return rect === null ? null : rect.origin();
}

/// A fitted group's top-left corner, which doesn't depend on its fitted size: `at`
/// minus the pin's bounding-box offset, which is 0 on a fitted axis.
function fittedFrame(r: Resolver, idx: number): Pt | null {
  const f = r.frames.get(idx);
  if (f !== undefined) {
    return f;
  }
  r.frames.set(idx, null);
  const item = r.entries[idx]!.item;
  if (item.kind !== "Group") {
    return null;
  }
  const origin = frameOrigin(r, idx);
  if (origin === null) return null;
  const at = evalPoint(r, item.value.node.at, origin);
  if (at === null) return null;
  const p = Entry.node(r.entries[idx]!);
  if (p === null || p.size === null) return null;
  const [w, h] = p.size;
  const [pinName, t] = p.pin;
  const unit = anchor.resolve(new Rect(0.0, 0.0, 1.0, 1.0), 0.0, "pin", pinName, t);
  if (!unit.ok) return null;
  const fr = new Pt(at.x - unit.value.x * w, at.y - unit.value.y * h);
  r.frames.set(idx, fr);
  return fr;
}

/// Places a fitted group: from its frame to the far edge of everything inside it, plus
/// `padding`, and at least as large as its title; with `title=bottom`, the title's
/// height goes below the far edge instead. A fitted axis is rounded up to whole
/// pixels, then to the `grid`.
function placeFitted(r: Resolver, idx: number, g: ast.Group): Rect | null {
  const origin = fittedFrame(r, idx);
  if (origin === null) return null;
  const reach = Reach.default();
  const [from, to] = descendants(r, idx);
  for (let d = from; d < to; d++) {
    ensureBuilt(r, d);
    const b = r.built.get(d);
    if (b === undefined || b === null) continue;
    switch (b.kind) {
      case "Shape":
        reach.shape(b.value);
        break;
      case "Junction":
        reach.junction(b.value);
        break;
      case "Text":
        reach.text(b.value);
        break;
      case "Line":
        reach.line(b.value);
        break;
    }
  }
  const p = Entry.node(r.entries[idx]!);
  if (p === null || p.size === null) return null;
  const [w, h] = p.size;
  const shape = drawnShape(p);
  const title = textlayout.fitSize({
    id: g.node.id.name,
    node: g.node,
    kind: "group",
    shape,
    customTextArea: null,
    textBelow: false,
    contain: null,
    iconInset: nodeIconInset(p, true),
    rect: new Rect(0.0, 0.0, w, h),
    style: p.style,
    props: p.text,
    blockAt: [],
  });
  const [farX, farY] = reach.far();
  const what = `\`${g.node.id.name}\``;
  if (farX === -Infinity && title === null) {
    r.diags.push(
      Diagnostic.error(
        "size",
        g.node.span,
        `group ${what} has no size to fit: it has no children or label`,
      ).withHelp("give it a size: `size (w,h)`"),
    );
    return null;
  }
  const pad = p.style.padding;
  const grid = r.grid;
  const fit = (far: number, o: number, pad: number, title: number | null, below: boolean) => {
    const reachV = far !== -Infinity ? Math.max(far - o, 0.0) : null;
    let v: number;
    if (reachV !== null && title !== null && below) {
      v = reachV + title;
    } else {
      v = Math.max(reachV === null ? 0.0 : reachV + pad, title ?? 0.0);
    }
    v = Math.ceil(v);
    return grid === null ? v : Math.ceil(v / grid) * grid;
  };
  const below = p.text.title === "bottom";
  const [fw, fh] = p.fitted;
  const size: [number, number] = [
    fw ? fit(farX, origin.x, pad.x, title === null ? null : title[0], false) : w,
    fh ? fit(farY, origin.y, pad.y, title === null ? null : title[1], below) : h,
  ];
  if (!sizeOk(r, size, g.node.span, what)) {
    return null;
  }
  const np = Entry.node(r.entries[idx]!);
  if (np !== null) {
    np.size = size;
  }
  return new Rect(origin.x, origin.y, size[0], size[1]);
}

export function evalPoint(r: Resolver, p: Point, origin: Pt): Pt | null {
  switch (p.kind) {
    case "Lit": {
      const x = evalCoord(r, p.x, origin.x);
      if (x === null) return null;
      const y = evalCoord(r, p.y, origin.y);
      if (y === null) return null;
      return finite(r, [x, y], p.span) ? new Pt(x, y) : null;
    }
    case "Ref": {
      const base = evalRef(r, p.r);
      if (base === null) return null;
      const [dx, dy] = p.offset === null ? [0.0, 0.0] : offsetDelta(p.offset);
      const q = base.add(new Pt(dx, dy));
      return finite(r, [q.x, q.y], p.span) ? q : null;
    }
    case "Group":
      return evalPoint(r, p.inner, origin);
    case "Shifted": {
      let q = evalPoint(r, p.base, origin);
      if (q === null) return null;
      for (const o of p.offsets) {
        const [dx, dy] = offsetDelta(o);
        q = q.add(new Pt(dx, dy));
      }
      return finite(r, [q.x, q.y], p.span) ? q : null;
    }
  }
}

export function evalCoord(r: Resolver, c: Coord, origin: number): number | null {
  if (c.kind === "Num") {
    return origin + c.value;
  }
  const p = evalRef(r, c.r);
  if (p === null) return null;
  return c.axis === "X" ? p.x : p.y;
}

export function evalTarget(r: Resolver, t: Target, origin: Pt, axis: Axis): number | null {
  if (t.kind === "Num") {
    return axis === "X" ? origin.x + t.value : origin.y + t.value;
  }
  const p = evalPoint(r, t.value, origin);
  if (p === null) return null;
  return axis === "X" ? p.x : p.y;
}

export function evalRef(r: Resolver, ref: ast.Ref): Pt | null {
  const nm = ref.id.name;
  const idx = r.ids.get(nm);
  if (idx === undefined) {
    if (r.skippedIds.has(nm)) {
      return null;
    }
    const d = unknownId(r, nm, ref.id.span);
    r.diags.push(d);
    return null;
  }
  const item = r.entries[idx]!.item;
  switch (item.kind) {
    case "Junction": {
      if (ref.anchor !== null) {
        r.diags.push(
          Diagnostic.error(
            "unknown-anchor",
            ref.anchor.name.span,
            `\`${nm}\` is a junction and has no anchors`,
          ).withHelp(`use \`${nm}\` on its own`),
        );
        return null;
      }
      const rect = place(r, idx);
      return rect === null ? null : rect.origin();
    }
    case "Box":
    case "Group":
    case "Image": {
      const a = ref.anchor;
      if (a === null) {
        r.diags.push(
          Diagnostic.error("unknown-anchor", ref.id.span, `\`${nm}\` needs an anchor`).withHelp(
            `e.g. \`${nm}.left\`, \`${nm}.top\` or \`${nm}.center\``,
          ),
        );
        return null;
      }
      const rect = place(r, idx);
      if (rect === null) return null;
      const p = Entry.node(r.entries[idx]!);
      if (p === null) return null;
      const f = p.shape === null ? null : formOf(r, p.shape, rect);
      const res = form.resolveAnchor(f, rect, p.style.radius, nm, a.name.name, a.t);
      if (res.ok) {
        return res.value;
      }
      const [msg, help] = res.error;
      const d = Diagnostic.error("unknown-anchor", a.name.span, msg);
      if (help !== null) {
        d.withHelp(help);
      }
      r.diags.push(d);
      return null;
    }
    case "Text":
    case "Line":
      r.diags.push(
        Diagnostic.error(
          "unknown-id",
          ref.id.span,
          `\`${nm}\` is a text or line; only boxes, groups and junctions can be referenced`,
        ),
      );
      return null;
  }
}

/// Strongly connected components of `deps`, each listed after every component it depends on.
export function tarjan(deps: readonly (readonly number[])[]): number[][] {
  const n = deps.length;
  const UNSET = -1;
  const index: number[] = new Array<number>(n).fill(UNSET);
  const low: number[] = new Array<number>(n).fill(0);
  const on: boolean[] = new Array<boolean>(n).fill(false);
  const stack: number[] = [];
  const sccs: number[][] = [];
  let counter = 0;
  for (let root = 0; root < n; root++) {
    if (index[root] !== UNSET) {
      continue;
    }
    const work: [number, number][] = [[root, 0]];
    while (work.length > 0) {
      const top = work[work.length - 1]!;
      const v = top[0];
      if (top[1] === 0 && index[v] === UNSET) {
        index[v] = counter;
        low[v] = counter;
        counter += 1;
        stack.push(v);
        on[v] = true;
      }
      const w = deps[v]![top[1]];
      if (w !== undefined) {
        top[1] += 1;
        if (index[w] === UNSET) {
          work.push([w, 0]);
        } else if (on[w]) {
          low[v] = Math.min(low[v]!, index[w]!);
        }
        continue;
      }
      work.pop();
      const u = work[work.length - 1];
      if (u !== undefined) {
        low[u[0]] = Math.min(low[u[0]]!, low[v]!);
      }
      if (low[v] === index[v]) {
        const scc: number[] = [];
        for (;;) {
          const x = stack.pop();
          if (x === undefined) break;
          on[x] = false;
          scc.push(x);
          if (x === v) {
            break;
          }
        }
        scc.sort((a, b) => a - b);
        sccs.push(scc);
      }
    }
  }
  return sccs;
}

/// The shortest cycle through `start` inside its component `scc`, starting at `start`.
export function shortestCycle(
  start: number,
  deps: readonly (readonly number[])[],
  scc: readonly number[],
): number[] {
  const prev = new Map<number, number>();
  const inScc = new Set(scc);
  const queue: number[] = [start];
  let qi = 0;
  while (qi < queue.length) {
    const v = queue[qi++]!;
    for (const w of deps[v]!) {
      if (w === start) {
        const cycle = [v];
        let cur = v;
        while (cur !== start) {
          cur = prev.get(cur)!;
          cycle.push(cur);
        }
        cycle.reverse();
        return cycle;
      }
      if (inScc.has(w) && !prev.has(w)) {
        prev.set(w, v);
        queue.push(w);
      }
    }
  }
  return [start];
}
