import * as os from "node:os";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

async function main() {
  const root = path.join(__dirname, "..", "..", "..");
  await runTests({
    vscodeExecutablePath: process.env.VSCODE_EXECUTABLE || undefined,
    extensionDevelopmentPath: root,
    extensionTestsPath: path.join(__dirname, "suite"),
    launchArgs: [
      "--disable-extensions",
      "--user-data-dir",
      path.join(os.tmpdir(), "kashika-vscode-test", "user"),
      "--extensions-dir",
      path.join(os.tmpdir(), "kashika-vscode-test", "extensions"),
    ],
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
