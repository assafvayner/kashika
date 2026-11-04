import { beforeEach, describe, expect, it } from "vitest";

import { deleteProject, listProjects, loadProject, saveProject } from "../src/persist/projects.js";
import { DEFAULT_SETTINGS, loadSettings, saveSettings } from "../src/persist/settings.js";
import { decodeShare, encodeShare } from "../src/persist/share.js";

describe("share", () => {
  it("round trips through the fragment", async () => {
    const src = 'diagram "héllo" size (300,200)\nbox a "A" at (20,20) size (80,40)\n'.repeat(20);
    const frag = await encodeShare(src);
    expect(frag).toMatch(/^k=[A-Za-z0-9_-]+$/);
    expect(frag.length).toBeLessThan(src.length);
    expect(await decodeShare(frag)).toBe(src);
    expect(await decodeShare("nothing")).toBeNull();
    expect(await decodeShare("k=!!!")).toBeNull();
  });
});

describe("settings", () => {
  beforeEach(() => localStorage.clear());
  it("defaults, saves and reloads", () => {
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
    saveSettings({ ...DEFAULT_SETTINGS, provider: "openai-compatible", budget: 5 });
    expect(loadSettings()).toMatchObject({ provider: "openai-compatible", budget: 5 });
    localStorage.setItem("kashika.settings", "{broken");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });
});

describe("projects", () => {
  it("saves, lists newest first, loads and deletes", async () => {
    await saveProject({
      id: "p1",
      title: "One",
      source: "a",
      chat: [],
      selection: null,
      updated: 1,
    });
    await saveProject({
      id: "p2",
      title: "Two",
      source: "b",
      chat: [],
      selection: "x",
      updated: 2,
    });
    expect((await listProjects()).map((p) => p.id)).toEqual(["p2", "p1"]);
    expect((await loadProject("p2"))?.selection).toBe("x");
    await deleteProject("p1");
    expect((await listProjects()).map((p) => p.id)).toEqual(["p2"]);
    expect(await loadProject("p1")).toBeNull();
  });
});
