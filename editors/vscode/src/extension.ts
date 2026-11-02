import * as path from "path";
import type MarkdownIt from "markdown-it";
import { cached, kashikaPlugin, Rendered, Render } from "./plugin";

interface Wasm {
  render(src: string, force: boolean): string;
}

let wasm: Wasm | undefined;

const render: Render = cached((src) => {
  // Loaded on first use: compiling the module takes a moment, and most previews have no diagram.
  wasm ??= require(path.join(__dirname, "..", "..", "wasm", "kashika_wasm.js")) as Wasm;
  return JSON.parse(wasm.render(src, true)) as Rendered;
});

export function activate() {
  return {
    extendMarkdownIt(md: MarkdownIt) {
      kashikaPlugin(md, render);
      return md;
    },
  };
}
