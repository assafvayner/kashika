import { readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join } from "node:path";
import { expect, test } from "vitest";

import { ROOT, kashika, pngSize, tempdir } from "../support/cli.js";
import { DIVERGENT_EXAMPLES } from "../golden/divergent.js";

const EXAMPLES = join(ROOT, "examples");

/// Extensions allowed in an example directory: source, renders and imported .kash files.
const ALLOWED_EXTENSIONS = ["svg", "png", "kash"];

/// The PNG renders are compared by size: the TypeScript build rasterizes with resvg-js, whose
/// encoder does not write the same bytes as the Rust one.
test("example_renders_are_committed_and_current", () => {
  const names = readdirSync(EXAMPLES)
    .filter((n) => statSync(join(EXAMPLES, n)).isDirectory())
    .sort();
  expect(names.length).toBeGreaterThan(0);
  expect([...DIVERGENT_EXAMPLES.keys()].filter((n) => !names.includes(n))).toEqual([]);

  const readme = readFileSync(join(EXAMPLES, "README.md"), "utf8");
  const tmp = tempdir();
  for (const name of names) {
    expect(
      readme.split("\n").some((l) => l === `## ${name}`),
      `examples/README.md has no \`## ${name}\` section`,
    ).toBe(true);
    const dir = join(EXAMPLES, name);
    const src = join(dir, `${name}.kash`);
    const regen = `regenerate with: for ext in svg png; do cargo run -p kashika -- render examples/${name}/${name}.kash -o examples/${name}/${name}.$ext; done`;

    const files = new Set(readdirSync(dir));
    for (const required of [`${name}.kash`, `${name}.svg`, `${name}.png`]) {
      expect(files.has(required), `examples/${name}/ is missing ${required}; ${regen}`).toBe(true);
    }
    for (const f of files) {
      expect(
        ALLOWED_EXTENSIONS.includes(extname(f).slice(1)),
        `unexpected file examples/${name}/${f}`,
      ).toBe(true);
    }

    const r = kashika(["check", src]);
    expect(r.code, name).toBe(0);
    expect(r.stdout).toContain("0 errors, 0 warnings");

    expect(kashika(["fmt", "--check", src]).code, name).toBe(0);

    for (const ext of ["svg", "png"]) {
      const out = join(tmp, `${name}.${ext}`);
      expect(kashika(["render", src, "-o", out]).code, name).toBe(0);
      const fresh = new Uint8Array(readFileSync(out));
      const committed = new Uint8Array(readFileSync(join(dir, `${name}.${ext}`)));
      if (ext === "svg") {
        const same = Buffer.from(fresh).equals(Buffer.from(committed));
        if (DIVERGENT_EXAMPLES.has(name)) {
          expect(same, `examples/${name} matches: remove it from divergent.ts`).toBe(false);
        } else {
          expect(same, `examples/${name}/${name}.${ext} is stale; ${regen}`).toBe(true);
        }
      } else {
        expect(pngSize(fresh), `examples/${name}/${name}.png`).toEqual(pngSize(committed));
      }
    }
  }
});
