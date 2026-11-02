import * as assert from "node:assert/strict";
import * as vscode from "vscode";

export async function run(): Promise<void> {
  const md = '# Doc\n\n```kashika\ndiagram "t" size (200,100)\nbox a "A" at (20,20) size (80,40)\n```\n';
  const html = await vscode.commands.executeCommand<string>("markdown.api.render", md);
  assert.match(html, /<div class="kashika-diagram code-line" data-line="2"><img alt="kashika diagram" src="data:image\/svg\+xml;base64,/);

  const lang = await vscode.languages.getLanguages();
  assert.ok(lang.includes("kashika"));
}
