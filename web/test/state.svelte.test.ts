import "fake-indexeddb/auto";
import { describe, expect, it } from "vitest";

import type { Session } from "../src/auth.js";
import { type Project, listProjects } from "../src/persist/projects.js";
import { type ChatEvent, type Provider, ProviderError } from "../src/providers/types.js";
import { AppState } from "../src/state.svelte.js";
import { engine } from "./wasm.js";

const SRC = 'diagram "t" size (300,200)\nbox a "A" at (20,20) size (80,40)\n';

function scripted(turns: ChatEvent[][], failWith?: Error): Provider {
  return {
    id: "openai-compatible",
    async listModels() {
      return [];
    },
    async *chat() {
      if (failWith) throw failWith;
      const t = turns.shift();
      if (t === undefined) throw new Error("no more turns");
      yield* t;
    },
  };
}

class TestState extends AppState {
  fake: Provider = scripted([]);
  override get provider(): Provider | null {
    return this.fake;
  }
  override get model(): string | null {
    return "m";
  }
}

const session: Session = { token: "t", expires: Date.now() + 3_600_000, name: "u" };

const turns = (): ChatEvent[][] => [
  [
    {
      type: "tool_call",
      call: {
        id: "c1",
        name: "write_source",
        arguments: JSON.stringify({ source: SRC + 'box b "B" at (200,20) size (80,40)\n' }),
      },
    },
    { type: "finish", reason: "tool_calls" },
  ],
  [
    { type: "text_delta", text: "Added B." },
    { type: "finish", reason: "stop" },
  ],
];

async function stored(id: string): Promise<Project | undefined> {
  return (await listProjects()).find((p) => p.id === id);
}

describe("AppState", () => {
  it("send appends a user and an assistant entry", async () => {
    const app = new TestState(engine, session);
    app.fake = scripted(turns());
    const p = app.send("add B");
    expect(app.running).toBe(true);
    await p;
    expect(app.running).toBe(false);
    expect(app.project.chat).toHaveLength(2);
    expect(app.project.chat[0]).toMatchObject({ role: "user", text: "add B" });
    const reply = app.project.chat[1]!;
    expect(reply.text).toContain("Added B.");
    expect(reply.steps).toHaveLength(1);
    expect(reply.steps![0]!.summary).toContain("write_source");
  });

  it("keeps stored history when saving an opened project", async () => {
    const app = new TestState(engine, null);
    app.open({
      id: "p1",
      title: "T",
      source: SRC,
      chat: [],
      selection: null,
      updated: 1,
      history: [{ role: "user", content: "hi" }],
    });
    await app.saveNow();
    expect((await stored("p1"))!.history).toEqual([{ role: "user", content: "hi" }]);
  });

  it("rename persists across saves and clearing returns to the diagram title", async () => {
    const app = new TestState(engine, session);
    app.open({ id: "r1", title: "x", source: SRC, chat: [], selection: null, updated: 1 });
    app.rename("Mine");
    await app.saveNow();
    expect(app.title).toBe("Mine");
    expect((await stored("r1"))!.title).toBe("Mine");
    expect((await stored("r1"))!.customTitle).toBe("Mine");
    app.rename("");
    await app.saveNow();
    expect(app.title).toBe("t");
    expect((await stored("r1"))!.title).toBe("t");
    expect((await stored("r1"))!.customTitle).toBeUndefined();
  });

  it("keeps the turn's history after the settings change", async () => {
    const app = new TestState(engine, session);
    app.fake = scripted(turns());
    await app.send("add B");
    app.updateSettings({ budget: 3 });
    await app.saveNow();
    const s = await stored(app.project.id);
    expect(JSON.stringify(s!.history)).toContain("add B");
  });

  it("logs out on a 401 from the provider", async () => {
    const app = new TestState(engine, session);
    app.fake = scripted([], new ProviderError("401 Unauthorized", 401));
    await app.send("hi");
    expect(app.session).toBeNull();
  });

  it("does not resurrect a deleted current project", async () => {
    const app = new TestState(engine, null);
    app.open({
      id: "gone",
      title: "G",
      source: SRC,
      chat: [],
      selection: null,
      updated: 1,
    });
    await app.saveNow();
    app.scheduleSave();
    await app.deleteProject("gone");
    await new Promise((r) => setTimeout(r, 700));
    expect((await listProjects()).map((p) => p.id)).not.toContain("gone");
  });
});
