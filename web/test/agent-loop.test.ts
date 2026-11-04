import { describe, expect, it } from "vitest";

import { Agent, type AgentEvent } from "../src/agent/loop.js";
import { Document } from "../src/document/store.svelte.js";
import type { ChatEvent, ChatRequest, Provider } from "../src/providers/types.js";
import { engine } from "./wasm.js";

const SRC = 'diagram "t" size (300,200)\nbox a "A" at (20,20) size (80,40)\n';

/** A provider that replays scripted turns; each turn is a list of events. */
function scripted(turns: ChatEvent[][]): Provider & { requests: ChatRequest[] } {
  const requests: ChatRequest[] = [];
  return {
    id: "openai-compatible",
    requests,
    async listModels() {
      return [];
    },
    async *chat(req) {
      requests.push(req);
      const t = turns.shift();
      if (t === undefined) throw new Error("no more turns");
      yield* t;
    },
  };
}

const write = (id: string, source: string): ChatEvent => ({
  type: "tool_call",
  call: { id, name: "write_source", arguments: JSON.stringify({ source }) },
});

async function run(agent: Agent, text: string): Promise<AgentEvent[]> {
  const out: AgentEvent[] = [];
  for await (const e of agent.run(text)) out.push(e);
  return out;
}

describe("Agent", () => {
  it("runs a tool turn then a final message", async () => {
    const p = scripted([
      [
        write("c1", SRC + 'box b "B" at (200,20) size (80,40)\n'),
        { type: "finish", reason: "tool_calls" },
      ],
      [
        { type: "text_delta", text: "Added B." },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const doc = new Document(engine, SRC);
    const agent = new Agent({ engine, doc, provider: p, model: "m", budget: 20 });
    const events = await run(agent, "add a box B");
    expect(doc.source).toContain("box b");
    expect(events.map((e) => e.type)).toEqual(["tool_start", "tool_result", "text", "done"]);
    expect(p.requests[0]!.messages[0]!.role).toBe("system");
    expect(p.requests[0]!.messages.at(-1)).toMatchObject({ role: "user" });
    expect((p.requests[0]!.messages.at(-1) as { content: string }).content).toContain(
      "add a box B",
    );
    expect((p.requests[0]!.messages.at(-1) as { content: string }).content).toContain(SRC);
    expect(p.requests[1]!.messages.some((m) => m.role === "tool")).toBe(true);
    expect(agent.history.at(-1)).toEqual({
      role: "assistant",
      content: "[write_source: 0 errors, 0 warnings]\nAdded B.",
    });
  });

  it("collapses tool results after the turn", async () => {
    const p = scripted([
      [write("c1", SRC), { type: "finish", reason: "tool_calls" }],
      [
        { type: "text_delta", text: "ok" },
        { type: "finish", reason: "stop" },
      ],
      [
        { type: "text_delta", text: "again" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const agent = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: p,
      model: "m",
      budget: 20,
    });
    await run(agent, "x");
    await run(agent, "y");
    const second = p.requests[2]!.messages;
    expect(second.some((m) => m.role === "tool")).toBe(false);
    expect(
      second.some((m) => m.role === "assistant" && (m.content ?? "").includes("[write_source")),
    ).toBe(true);
  });

  it("stops at the budget", async () => {
    const turns = Array.from({ length: 5 }, (_, i) => [
      write(`c${i}`, SRC + `// ${i}\n`),
      { type: "finish", reason: "tool_calls" } as ChatEvent,
    ]);
    const p = scripted(turns);
    const agent = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: p,
      model: "m",
      budget: 2,
    });
    const events = await run(agent, "loop");
    expect(events.at(-1)).toMatchObject({ type: "stopped", steps: 2 });
    expect(p.requests.length).toBe(3);
  });

  it("includes the selection", async () => {
    const p = scripted([
      [
        { type: "text_delta", text: "ok" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const doc = new Document(engine, SRC);
    doc.selection = "a";
    await run(new Agent({ engine, doc, provider: p, model: "m", budget: 2 }), "move this");
    expect((p.requests[0]!.messages.at(-1) as { content: string }).content).toContain(
      "selected: a",
    );
  });

  it("retries once on an empty reply, then errors", async () => {
    const p = scripted([
      [{ type: "finish", reason: "stop" }],
      [{ type: "finish", reason: "stop" }],
    ]);
    const events = await run(
      new Agent({ engine, doc: new Document(engine, SRC), provider: p, model: "m", budget: 5 }),
      "x",
    );
    expect(events.at(-1)).toMatchObject({ type: "error" });
    expect(p.requests.length).toBe(2);
  });

  it("aborts", async () => {
    const p: Provider = {
      id: "openai-compatible",
      async listModels() {
        return [];
      },
      async *chat(_req, signal) {
        await new Promise((_, reject) =>
          signal.addEventListener("abort", () => reject(new DOMException("x", "AbortError"))),
        );
      },
    };
    const agent = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: p,
      model: "m",
      budget: 5,
    });
    const events: AgentEvent[] = [];
    const running = (async () => {
      for await (const e of agent.run("x")) events.push(e);
    })();
    agent.stop();
    await running;
    expect(events.at(-1)).toMatchObject({ type: "stopped" });
  });
});
