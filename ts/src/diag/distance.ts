import { chars, cmpStr } from "./rust.js";

/// The candidate closest to `name` that is at least one edit away (an insertion, deletion,
/// substitution or swap of neighbors), at most a third of the characters of the longer of
/// the two away (counting at least 3, so one edit up to five characters, two up to eight),
/// and fewer edits away than it has characters, so a one-letter candidate is never suggested.
export function didYouMean(name: string, candidates: Iterable<string>): string | null {
  const nameLen = chars(name).length;
  let best: [number, string] | null = null;
  for (const c of candidates) {
    const d = editDistance(name, c);
    const len = chars(c).length;
    if (!(d > 0 && d < len && d <= Math.floor(Math.max(len, nameLen, 3) / 3))) continue;
    if (best === null || d < best[0] || (d === best[0] && cmpStr(c, best[1]) < 0)) {
      best = [d, c];
    }
  }
  return best === null ? null : best[1];
}

export function editDistance(a: string, b: string): number {
  const ac = chars(a);
  const bc = chars(b);
  let older: number[] = new Array<number>(bc.length + 1).fill(0);
  let prev: number[] = Array.from({ length: bc.length + 1 }, (_, j) => j);
  for (let i = 1; i <= ac.length; i++) {
    const cur: number[] = new Array<number>(bc.length + 1).fill(i);
    for (let j = 1; j <= bc.length; j++) {
      const cost = ac[i - 1] !== bc[j - 1] ? 1 : 0;
      cur[j] = Math.min(prev[j]! + 1, cur[j - 1]! + 1, prev[j - 1]! + cost);
      if (i > 1 && j > 1 && ac[i - 1] === bc[j - 2] && ac[i - 2] === bc[j - 1]) {
        cur[j] = Math.min(cur[j]!, older[j - 2]! + 1);
      }
    }
    older = prev;
    prev = cur;
  }
  return prev[bc.length]!;
}
