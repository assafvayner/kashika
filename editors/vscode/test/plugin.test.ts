import { test } from "node:test";
import * as assert from "node:assert/strict";
import * as path from "node:path";
import MarkdownIt from "markdown-it";
import { cached, kashikaPlugin, Rendered } from "../src/plugin";

const wasm = require(path.join(__dirname, "..", "..", "wasm", "kashika_wasm.js"));
const render = (src: string): Rendered => JSON.parse(wasm.render(src, true));

function md() {
  const m = new MarkdownIt();
  kashikaPlugin(m, render);
  return m;
}

const CLEAN = 'diagram "t" size (200,100)\nbox a "A" at (20,20) size (80,40)\n';

test("renders a kashika fence as an inline SVG image", () => {
  const html = md().render("# Title\n\n```kashika\n" + CLEAN + "```\n");
  const src = html.match(/src="data:image\/svg\+xml;base64,([^"]+)"/);
  assert.ok(src, html);
  assert.ok(Buffer.from(src[1], "base64").toString().startsWith("<svg"));
  assert.match(html, /class="kashika-diagram code-line" data-line="2"/);
  assert.doesNotMatch(html, /kashika-diagnostics/);
});

test("accepts the kash alias", () => {
  assert.match(md().render("```kash\n" + CLEAN + "```\n"), /data:image\/svg\+xml/);
});

test("leaves other fences alone", () => {
  assert.equal(md().render("```rust\nfn main() {}\n```\n"), new MarkdownIt().render("```rust\nfn main() {}\n```\n"));
});

test("lists diagnostics with Markdown line numbers", () => {
  const src = 'diagram "t" size (200,100)\nbox a "A" at (20,20) size (80,40) icon="missing.svg"\n';
  const html = md().render("text\n\n```kashika\n" + src + "```\n");
  assert.match(html, /<div class="kashika-error">5:\d+ error\[file\]: file `missing.svg` not found<\/div>/);
});

test("escapes diagnostic text", () => {
  const html = md().render('```kashika\ndiagram "<b>" size (10,10)\nbogus <b>\n```\n');
  assert.doesNotMatch(html, /<b>/);
});

test("reports a render failure instead of throwing", () => {
  const m = new MarkdownIt();
  kashikaPlugin(m, () => {
    throw new Error("boom");
  });
  assert.match(m.render("```kashika\nx\n```\n"), /kashika failed: Error: boom/);
});

test("cached renders each source once", () => {
  let calls = 0;
  const r = cached((src) => {
    calls++;
    return render(src);
  });
  r(CLEAN);
  r(CLEAN);
  assert.equal(calls, 1);
});

test("cached keeps recently used sources", () => {
  let calls = 0;
  const r = cached((src) => {
    calls++;
    return render(src);
  });
  r(CLEAN);
  for (let i = 0; i < 63; i++) r(`diagram "d${i}" size (10,10)\n`);
  r(CLEAN);
  r(`diagram "new" size (10,10)\n`);
  r(CLEAN);
  assert.equal(calls, 65);
});

test("explains a fence with nothing to render", () => {
  const m = new MarkdownIt();
  kashikaPlugin(m, () => ({ svg: null, errors: 0, warnings: 0, diagnostics: [] }));
  assert.match(m.render("```kashika\nstyle s { fill=\"#fff\" }\n```\n"), /nothing to render/);
});
