import { test } from "node:test";
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";

const pkg = path.join(__dirname, "..", "..", "wasm");
const repo = path.join(__dirname, "..", "..", "..", "..");

function loadWasm(): { render(src: string, force: boolean): string } {
  for (const file of ["kashika_wasm.js", "kashika_wasm_bg.wasm"]) {
    if (!fs.existsSync(path.join(pkg, file))) {
      assert.fail(`${path.join(pkg, file)} is missing: build the wasm package with \`npm run build:wasm\` (or \`npm run build\`) in editors/vscode first`);
    }
  }
  return require(path.join(pkg, "kashika_wasm.js"));
}

test("the wasm render of the upload example matches the CLI render byte for byte", () => {
  const wasm = loadWasm();
  const src = fs.readFileSync(path.join(repo, "examples", "upload", "upload.kash"), "utf8");
  const want = fs.readFileSync(path.join(repo, "examples", "upload", "upload.svg"), "utf8");
  const r = JSON.parse(wasm.render(src, false));
  assert.equal(r.errors, 0, JSON.stringify(r.diagnostics));
  assert.ok(r.svg === want, "wasm SVG differs from examples/upload/upload.svg");
});
