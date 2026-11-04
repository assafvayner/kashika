import { loadEngine } from "../src/engine/index.js";

interface NodeFs {
  readFileSync(path: string): Uint8Array;
}

const fsModule = "node:fs";
const { readFileSync } = (await import(/* @vite-ignore */ fsModule)) as NodeFs;
const { cwd } = (globalThis as unknown as { process: { cwd(): string } }).process;

export const engine = await loadEngine(
  readFileSync(`${cwd()}/src/engine/pkg/kashika_wasm_bg.wasm`),
);
