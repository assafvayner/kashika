type Op = { kind: " " | "-" | "+"; line: string; a: number; b: number };

function splitLines(s: string): string[] {
  const out = s.split("\n").map((l, i, all) => (i < all.length - 1 ? `${l}\n` : l));
  if (out[out.length - 1] === "") out.pop();
  return out;
}

/// The LCS edit script from `a` to `b`, with common prefix and suffix trimmed first so
/// large, mostly equal files stay cheap.
function edits(a: string[], b: string[]): Op[] {
  let pre = 0;
  while (pre < a.length && pre < b.length && a[pre] === b[pre]) pre++;
  let suf = 0;
  while (
    suf < a.length - pre &&
    suf < b.length - pre &&
    a[a.length - 1 - suf] === b[b.length - 1 - suf]
  ) {
    suf++;
  }
  const x = a.slice(pre, a.length - suf);
  const y = b.slice(pre, b.length - suf);
  const ops: Op[] = [];
  for (let i = 0; i < pre; i++) ops.push({ kind: " ", line: a[i]!, a: i, b: i });
  if (x.length * y.length > 25_000_000) {
    x.forEach((l, i) => ops.push({ kind: "-", line: l, a: pre + i, b: pre }));
    y.forEach((l, j) => ops.push({ kind: "+", line: l, a: pre + x.length, b: pre + j }));
  } else {
    const n = x.length;
    const m = y.length;
    const t: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
    for (let i = n - 1; i >= 0; i--) {
      for (let j = m - 1; j >= 0; j--) {
        t[i]![j] = x[i] === y[j] ? t[i + 1]![j + 1]! + 1 : Math.max(t[i + 1]![j]!, t[i]![j + 1]!);
      }
    }
    let i = 0;
    let j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && x[i] === y[j]) {
        ops.push({ kind: " ", line: x[i]!, a: pre + i, b: pre + j });
        i++;
        j++;
      } else if (j < m && (i === n || t[i]![j + 1]! >= t[i + 1]![j]!)) {
        ops.push({ kind: "+", line: y[j]!, a: pre + i, b: pre + j });
        j++;
      } else {
        ops.push({ kind: "-", line: x[i]!, a: pre + i, b: pre + j });
        i++;
      }
    }
  }
  for (let k = 0; k < suf; k++) {
    ops.push({
      kind: " ",
      line: a[a.length - suf + k]!,
      a: a.length - suf + k,
      b: b.length - suf + k,
    });
  }
  return ops;
}

/// A unified line diff of `expected` and `actual` with `context` lines around each change.
export function unifiedDiff(expected: string, actual: string, context: number): string {
  const ops = edits(splitLines(expected), splitLines(actual));
  let out = "--- expected\n+++ actual\n";
  let k = 0;
  while (k < ops.length) {
    if (ops[k]!.kind === " ") {
      k++;
      continue;
    }
    const start = Math.max(0, k - context);
    let end = k;
    let last = k;
    while (end < ops.length && end - last <= 2 * context) {
      if (ops[end]!.kind !== " ") last = end;
      end++;
    }
    end = Math.min(ops.length, last + context + 1);
    const hunk = ops.slice(start, end);
    const aLen = hunk.filter((o) => o.kind !== "+").length;
    const bLen = hunk.filter((o) => o.kind !== "-").length;
    out += `@@ -${hunk[0]!.a + 1},${aLen} +${hunk[0]!.b + 1},${bLen} @@\n`;
    for (const o of hunk) {
      out += `${o.kind}${o.line}${o.line.endsWith("\n") ? "" : "\n\\ No newline at end of file\n"}`;
    }
    k = end;
  }
  return out;
}
