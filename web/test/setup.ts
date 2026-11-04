import "fake-indexeddb/auto";

if (typeof globalThis.localStorage?.clear !== "function") {
  const data = new Map<string, string>();
  const storage = {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, String(v)),
    removeItem: (k: string) => void data.delete(k),
    clear: () => data.clear(),
  };
  Object.defineProperty(globalThis, "localStorage", { value: storage, configurable: true });
}

globalThis.DOMMatrix ??= class {
  a = 1;
  b = 0;
  c = 0;
  d = 1;
  e = 0;
  f = 0;
  inverse() {
    return this;
  }
} as unknown as typeof DOMMatrix;
globalThis.DOMPoint ??= class {
  constructor(
    public x = 0,
    public y = 0,
  ) {}
  matrixTransform() {
    return this;
  }
} as unknown as typeof DOMPoint;
