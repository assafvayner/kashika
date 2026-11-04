import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, test } from "vitest";

import { type Found, Loader, Options, pathJoin, pathParent } from "../../src/model/loader.js";

const temps: string[] = [];

afterEach(() => {
  for (const t of temps.splice(0)) rmSync(t, { recursive: true, force: true });
});

function setup(allowPath: boolean): [string, Loader] {
  const tmp = mkdtempSync(join(tmpdir(), "kashika-loader-"));
  temps.push(tmp);
  const proj = join(tmp, "proj");
  mkdirSync(join(proj, "icons"), { recursive: true });
  writeFileSync(join(proj, "icons/s3.svg"), "<svg/>");
  writeFileSync(join(tmp, "secret.svg"), "<svg/>");
  const l = Loader.new(new Options({ path: join(proj, "main.kash"), allowPath }));
  return [tmp, l];
}

function found(l: Loader, name: string): Found {
  const r = l.find(l.root(), name);
  if (!r.ok) throw new Error(r.error.message);
  return r.value;
}

function error(l: Loader, name: string) {
  const r = l.find(l.root(), name);
  if (r.ok) throw new Error(`found ${name}`);
  return r.error;
}

describe("loader", () => {
  test("finds_relative_files", () => {
    const [, l] = setup(false);
    const f = found(l, "icons/s3.svg");
    expect(f.kind === "file" && f.path.endsWith(join("icons", "s3.svg"))).toBe(true);
  });

  test("bundled_icons_by_bare_name", () => {
    const [, l] = setup(false);
    expect(found(l, "server")).toEqual({ kind: "bundled", name: "server" });
    expect(error(l, "servr").help).toBe("did you mean `server`?");
  });

  test("missing_files_suggest_a_near_name", () => {
    const [, l] = setup(false);
    const e = error(l, "icons/s4.svg");
    expect(e.message).toBe("file `icons/s4.svg` not found");
    expect(e.help).toBe("did you mean `icons/s3.svg`?");
  });

  test("suggestions_stay_inside_the_sandbox", () => {
    let [, l] = setup(false);
    let e = error(l, "../secrt.svg");
    expect(e.message).toBe("file `../secrt.svg` not found");
    expect(e.help).toBeNull();
    [, l] = setup(true);
    e = error(l, "../secrt.svg");
    expect(e.help).toBe("did you mean `../secret.svg`?");
  });

  test("directories_are_named_as_such", () => {
    const [, l] = setup(false);
    expect(error(l, "icons").message).toBe("`icons` is a directory, not a file");
    const e = error(l, "icon");
    expect(e.message).toBe("file `icon` not found");
    expect(e.help).toBeNull();
  });

  test("escaping_the_root_needs_allow_path", () => {
    let [, l] = setup(false);
    expect(error(l, "../secret.svg").message).toContain("outside the project root");
    [, l] = setup(true);
    expect(l.find(l.root(), "../secret.svg").ok).toBe(true);
  });

  test.skipIf(process.platform === "win32")("symlinks_out_of_the_root_are_rejected", () => {
    const [t] = setup(false);
    const proj = join(t, "proj");
    symlinkSync(join(t, "secret.svg"), join(proj, "link.svg"));
    const l = Loader.new(new Options({ path: join(proj, "main.kash") }));
    expect(error(l, "link.svg").message).toContain("outside the project root");
    expect(error(l, "lnk.svg").help ?? "").not.toContain("link");
  });

  test("reads_are_capped", () => {
    const [, l] = setup(false);
    const f = found(l, "icons/s3.svg");
    const r = l.read(f, "icons/s3.svg", 6);
    expect(r.ok && new TextDecoder().decode(r.value)).toBe("<svg/>");
    const e = l.read(f, "icons/s3.svg", 5);
    expect(!e.ok && e.error.message).toBe("`icons/s3.svg` is larger than 5 bytes");
    const server = found(l, "server");
    expect(l.read(server, "server", 0).ok).toBe(true);
  });

  test("urls_are_rejected", () => {
    const [, l] = setup(false);
    expect(error(l, "https://example.com/x.svg").message).toContain("not a local file");
  });

  test("search_path_from_options_and_icons_dir", () => {
    const [t] = setup(false);
    const proj = join(t, "proj");
    let l = Loader.new(
      new Options({ path: join(proj, "main.kash"), iconPath: [join(proj, "icons")] }),
    );
    expect(l.find(l.root(), "s3.svg").ok).toBe(true);
    l = Loader.new(new Options({ path: join(proj, "main.kash") }));
    expect(l.find(l.root(), "s3.svg").ok).toBe(false);
    expect(l.addIconsDir("icons").ok).toBe(true);
    expect(l.find(l.root(), "s3.svg").ok).toBe(true);
    expect(l.addIconsDir("..").ok).toBe(false);
    expect(l.addIconsDir("nope").ok).toBe(false);
  });

  test("included_files_are_allowed_one_by_one", () => {
    const [t] = setup(false);
    writeFileSync(join(t, "other.svg"), "<svg/>");
    const l = Loader.new(
      new Options({
        path: join(t, "proj/main.kash"),
        include: [join(t, "proj") + "/../secret.svg"],
      }),
    );
    expect(l.find(l.root(), "../secret.svg").ok).toBe(true);
    let e = error(l, "../other.svg");
    expect(e.message).toContain("outside the project root");
    expect(e.help ?? "").toContain("--include");
    e = error(l, "../secrt.svg");
    expect(e.help).toBe("did you mean `../secret.svg`?");
  });

  test("env_search_dirs_are_trusted_roots", () => {
    const [t] = setup(false);
    const l = Loader.new(new Options({ path: join(t, "proj/main.kash"), iconPath: [t] }));
    expect(l.find(l.root(), "secret.svg").ok).toBe(true);
  });

  test("paths_join_and_split_like_rust", () => {
    expect(pathJoin("a", "b")).toBe("a/b");
    expect(pathJoin("a/", "b")).toBe("a/b");
    expect(pathJoin("", "b")).toBe("b");
    expect(pathJoin("a", "/b")).toBe("/b");
    expect(pathJoin("a", "../b")).toBe("a/../b");
    expect(pathParent("icons/s4.svg")).toBe("icons");
    expect(pathParent("s4.svg")).toBe("");
    expect(pathParent("../x")).toBe("..");
    expect(pathParent("/x")).toBe("/");
    expect(pathParent("/")).toBeNull();
    expect(pathParent("")).toBeNull();
    expect(pathParent("a/b/")).toBe("a");
  });

  test("io_errors_display_like_rust", () => {
    const [t, l] = setup(false);
    const r = l.read({ kind: "file", path: join(t, "nope.svg") }, "nope.svg", 10);
    expect(!r.ok && r.error.message).toBe(
      "cannot read `nope.svg`: No such file or directory (os error 2)",
    );
  });
});
