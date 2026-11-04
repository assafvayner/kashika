import { type Diagnostic, order } from "../diag/diagnostic.js";
import { cmpTuple } from "../diag/rust.js";
import { code } from "../diag/codes.js";
import { type Scene } from "../model/index.js";
import { type Deferred, Finding } from "./free.js";
import { type Metrics, metricsWith } from "./metrics.js";
import { Suppressor } from "./suppress.js";

/// How many findings with one code and suppression were counted but not kept.
export interface Omitted {
  code: string;
  suppressed: boolean;
  count: number;
}

/// What `checkBounded` reports.
export class Bounded {
  constructor(
    public diags: Diagnostic[] = [],
    /// The help still to be worked out for each of `diags`, in the same order.
    public deferred: Deferred[][] = [],
    /// Findings past the limit, per code and suppression, in code order.
    public omitted: Omitted[] = [],
    /// Every finding per code, including those filtered out or omitted.
    private foundBy: Map<string, number> = new Map(),
  ) {}

  /// How many findings with `code` the lints made, including those filtered out or omitted.
  found(code: string): number {
    return this.foundBy.get(code) ?? 0;
  }

  /// Like `metrics`, counting every crossing found, also those filtered out or omitted.
  metrics(scene: Scene): Metrics {
    return metricsWith(scene, this.found(code.CROSS));
  }
}

class Bucket {
  /// Kept findings with the order they were pushed in.
  kept: [number, Finding][] = [];
  omitted = 0;

  /// Keeps the first `keep` findings in `sort` order.
  trim(keep: number): void {
    if (this.kept.length <= keep) {
      return;
    }
    this.kept.sort((a, b) => order(a[1].diag, b[1].diag) || a[0] - b[0]);
    this.omitted += this.kept.length - keep;
    this.kept.length = keep;
  }
}

/// Where lints put their findings: marks suppression, drops what `filter` rejects and keeps at
/// most `keep` findings per code and suppression, holding at most twice that at any time.
export class Sink {
  private suppressorV: Suppressor;
  private pushed = 0;
  private buckets = new Map<string, [string, boolean, Bucket]>();
  private foundBy = new Map<string, number>();

  constructor(
    scene: Scene,
    private keep: number,
    private filter: ((d: Diagnostic) => boolean) | null,
  ) {
    this.suppressorV = new Suppressor(scene);
  }

  push(f0: Finding | Diagnostic): void {
    const f = Finding.from(f0);
    const d = f.diag;
    this.suppressorV.apply(d);
    this.foundBy.set(d.code, (this.foundBy.get(d.code) ?? 0) + 1);
    if (this.filter !== null && !this.filter(d)) {
      return;
    }
    const key = `${d.suppressed ? 1 : 0}${d.code}`;
    let entry = this.buckets.get(key);
    if (entry === undefined) {
      entry = [d.code, d.suppressed, new Bucket()];
      this.buckets.set(key, entry);
    }
    const b = entry[2];
    if (this.keep === 0) {
      b.omitted += 1;
      return;
    }
    b.kept.push([this.pushed, f]);
    this.pushed += 1;
    if (b.kept.length >= this.keep * 2) {
      b.trim(this.keep);
    }
  }

  suppressor(): Suppressor {
    return this.suppressorV;
  }

  finish(): Bounded {
    const kept: [number, Finding][] = [];
    const omitted: Omitted[] = [];
    const buckets = [...this.buckets.values()].sort((a, b) => cmpTuple([a[0], a[1]], [b[0], b[1]]));
    for (const [code, suppressed, b] of buckets) {
      b.trim(this.keep);
      for (const k of b.kept) kept.push(k);
      if (b.omitted > 0) {
        omitted.push({ code, suppressed, count: b.omitted });
      }
    }
    kept.sort((a, b) => a[0] - b[0]);
    const found = new Map([...this.foundBy.entries()]);
    return new Bounded(
      kept.map(([, f]) => f.diag),
      kept.map(([, f]) => f.deferred),
      omitted,
      found,
    );
  }
}
