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

/** Event types without the per-call `model_start` markers. */
const types = (events: AgentEvent[]) =>
  events.map((e) => e.type).filter((t) => t !== "model_start");

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
    expect(types(events)).toEqual(["tool_start", "tool_result", "text", "done"]);
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

  it("forwards reasoning as thinking events and marks each model call", async () => {
    const doc = new Document(engine, SRC);
    const provider = scripted([
      [{ type: "reasoning_delta", text: "plan" }, write("c1", SRC)],
      [
        { type: "reasoning_delta", text: "done" },
        { type: "text_delta", text: "ok" },
      ],
    ]);
    const agent = new Agent({ engine, doc, provider, model: "m", budget: 5 });
    const events = await run(agent, "go");
    expect(events.map((ev) => ev.type)).toEqual([
      "model_start",
      "thinking",
      "tool_start",
      "tool_result",
      "model_start",
      "thinking",
      "text",
      "done",
    ]);
    expect(agent.history.at(-1)?.content).not.toContain("plan");
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
    expect(agent.history.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect((agent.history.at(-1) as { content: string }).content).toContain(
      "[stopped: step budget after 2 steps]",
    );
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
    const agent = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: p,
      model: "m",
      budget: 5,
    });
    const events = await run(agent, "x");
    expect(events.at(-1)).toMatchObject({ type: "error" });
    expect((events.at(-1) as { message: string }).message).toContain("Try another model");
    expect(p.requests.length).toBe(2);
    expect(p.requests[1]!.messages.at(-1)).toMatchObject({
      role: "user",
      content: expect.stringContaining("empty"),
    });
    expect(agent.history.map((m) => m.role)).toEqual(["user", "assistant"]);
    expect((agent.history.at(-1) as { content: string }).content).toContain("[error:");
  });

  it("aborts", async () => {
    let calls = 0;
    const p: Provider = {
      id: "openai-compatible",
      async listModels() {
        return [];
      },
      async *chat(_req, signal) {
        calls += 1;
        if (calls === 1) {
          yield write("c1", SRC);
          yield { type: "finish", reason: "tool_calls" };
          return;
        }
        if (signal.aborted) throw new DOMException("x", "AbortError");
        throw new Error("expected an aborted signal");
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
    for await (const e of agent.run("x")) {
      events.push(e);
      if (e.type === "tool_start") agent.stop();
    }
    expect(events.at(-1)).toMatchObject({ type: "stopped", steps: 1 });
    expect(agent.history.at(-1)).toMatchObject({ role: "assistant" });
    expect((agent.history.at(-1) as { content: string }).content).toContain("[stopped by user]");
  });

  it("answers invalid tool arguments with an error result", async () => {
    const p = scripted([
      [
        { type: "tool_call", call: { id: "c1", name: "write_source", arguments: "{bad" } },
        { type: "finish", reason: "tool_calls" },
      ],
      [
        { type: "text_delta", text: "sorry" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const doc = new Document(engine, SRC);
    const agent = new Agent({ engine, doc, provider: p, model: "m", budget: 5 });
    const events = await run(agent, "x");
    expect(doc.source).toBe(SRC);
    expect(events.filter((e) => e.type !== "model_start")[1]).toMatchObject({
      type: "tool_result",
      summary: "[write_source: invalid arguments]",
    });
    const tool = p.requests[1]!.messages.find((m) => m.role === "tool") as { content: string };
    expect(tool.content).toContain("arguments are not valid JSON");
    expect(events.at(-1)).toMatchObject({ type: "done" });
  });

  it("rejects a second run while busy", async () => {
    const p = scripted([
      [
        { type: "text_delta", text: "ok" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const agent = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: p,
      model: "m",
      budget: 5,
    });
    const first = agent.run("a");
    const firstEvents: AgentEvent[] = [];
    const next = await first.next();
    firstEvents.push(next.value as AgentEvent);
    const second = await run(agent, "b");
    expect(second).toEqual([{ type: "error", message: "agent is busy" }]);
    for await (const e of first) firstEvents.push(e);
    expect(firstEvents.at(-1)).toMatchObject({ type: "done" });
    expect(agent.history).toHaveLength(2);
  });
});
