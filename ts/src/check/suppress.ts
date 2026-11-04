import { type Diagnostic, type Element, code } from "../diag/index.js";
import { type Allow, type Scene } from "../model/index.js";
import { lineEl, shapeEl } from "./element.js";

/// Marks findings silenced by `allow=` on the diagram, or on a line, box or group that the
/// finding names. `allow=code:other` also requires `other` among the finding's elements. The
/// reason is that of the first such line, box or group (lines first, in file order) that
/// gives one, also when the diagram silences the code too.
export function suppress(scene: Scene, diags: Diagnostic[]): void {
  const s = new Suppressor(scene);
  for (const d of diags) {
    s.apply(d);
  }
}

/// Whether the `allow=` entry `code` (with `other` for `code:other`) on the element `owner`
/// suppresses `d`: the codes match, `d` names `owner`, and also `other` when given.
export function suppresses(
  code: string,
  other: string | null,
  owner: string,
  d: Diagnostic,
): boolean {
  return code === d.code && names(d, owner) && (other === null || (other !== owner && names(d, other)));
}

function names(d: Diagnostic, id: string): boolean {
  return d.elements.some((e) => e.id === id);
}

/// A line, box or group with an `allow=`.
export interface Owner {
  id: string;
  allow: Allow[];
  reason: string | null;
  el: Element;
}

export class Suppressor {
  private diagram: Allow[];
  /// Lines, then shapes, with an `allow=`.
  private owners: Owner[];
  /// Index into `owners` of the first owner with each id.
  private byId: Map<string, number>;
  /// Per owner, whether each `allow=` entry has suppressed a finding.
  private used: boolean[][];

  constructor(scene: Scene) {
    const owners: Owner[] = [
      ...scene.lines
        .filter((l) => l.allow.length > 0)
        .map((l) => ({ id: l.id, allow: l.allow, reason: l.reason, el: lineEl(l, null) })),
      ...scene.shapes
        .filter((s) => s.allow.length > 0)
        .map((s) => ({ id: s.id, allow: s.allow, reason: s.reason, el: shapeEl(s) })),
    ];
    const byId = new Map<string, number>();
    owners.forEach((o, i) => {
      if (!byId.has(o.id)) byId.set(o.id, i);
    });
    this.diagram = scene.allow;
    this.owners = owners;
    this.byId = byId;
    this.used = owners.map((o) => o.allow.map(() => false));
  }

  apply(d: Diagnostic): void {
    const byDiagram = this.diagram.some((a) => a.code === d.code);
    if (this.owners.length === 0) {
      d.suppressed ||= byDiagram;
      return;
    }
    const by: number[] = [];
    for (const e of d.elements) {
      const i = this.byId.get(e.id);
      if (i === undefined) {
        continue;
      }
      const o = this.owners[i]!;
      o.allow.forEach((a, j) => {
        if (suppresses(a.code, a.other, o.id, d)) {
          this.used[i]![j] = true;
          by.push(i);
        }
      });
    }
    by.sort((a, b) => a - b);
    d.suppressed ||= byDiagram || by.length > 0;
    for (const i of by) {
      const r = this.owners[i]!.reason;
      if (r !== null) {
        d.reason = r;
        break;
      }
    }
  }

  /// The `allow=` entries that have suppressed nothing so far, with their owner and whether
  /// none of that owner's entries has. `allow=unused-allow` is never listed.
  unused(): [Owner, Allow, boolean][] {
    const out: [Owner, Allow, boolean][] = [];
    this.owners.forEach((o, i) => {
      if (this.byId.get(o.id) !== i) return;
      const used = this.used[i]!;
      const none = !used.includes(true);
      o.allow.forEach((a, j) => {
        if (!used[j] && a.code !== code.UNUSED_ALLOW) out.push([o, a, none]);
      });
    });
    return out;
  }
}
