import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { loadEngine } from "../src/engine/index.js";

const wasmPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../src/engine/pkg/kashika_wasm_bg.wasm",
);

export const engine = await loadEngine(readFileSync(wasmPath));
