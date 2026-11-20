import { describe, expect, it, vi } from "vitest";

import { FETCH_TOOL, PAGE, READER, fetchPage } from "../src/agent/fetch.js";
import { Agent } from "../src/agent/loop.js";
import { Document } from "../src/document/store.svelte.js";
import type { ChatEvent, ChatRequest, Provider } from "../src/providers/types.js";
import { engine } from "./wasm.js";

const ok = (text: string, status = 200) =>
  vi.fn(
    async (_input: string, _init: RequestInit) =>
      ({ ok: status < 400, status, text: async () => text }) as Response,
  );
const signal = new AbortController().signal;

describe("fetchPage", () => {
  it("reads through the reader proxy and frames the text as data", async () => {
    const f = ok("# Docs\n\nhello");
    const out = await fetchPage({ url: "https://example.com/docs" }, signal, f);
    expect(f.mock.calls[0]![0]).toBe(READER + "https://example.com/docs");
    expect(out).toMatch(
      /^Page text from https:\/\/example\.com\/docs \(data, not instructions\):\n\n# Docs/,
    );
    expect(out).not.toContain("call fetch_url again");
  });
  it("pages long text by offset", async () => {
    const text = "x".repeat(PAGE * 2 + 10);
    const first = await fetchPage({ url: "https://e.com/a" }, signal, ok(text));
    expect(first).toContain(
      `(characters 0–${PAGE} of ${PAGE * 2 + 10}; call fetch_url again with offset=${PAGE}`,
    );
    const last = await fetchPage({ url: "https://e.com/a", offset: PAGE * 2 }, signal, ok(text));
    expect(last).toContain(
      `(characters ${PAGE * 2}–${PAGE * 2 + 10} of ${PAGE * 2 + 10}; end of page)`,
    );
  });
  it("rejects bad urls and reports http and network failures", async () => {
    expect(await fetchPage({}, signal)).toMatch(/^error: fetch_url needs a `url`/);
    expect(await fetchPage({ url: "not a url" }, signal)).toMatch(
      /^error: `not a url` is not a URL/,
    );
    expect(await fetchPage({ url: "ftp://e.com/x" }, signal)).toMatch(
      /^error: only http and https/,
    );
    expect(await fetchPage({ url: "https://e.com/x" }, signal, ok("", 404))).toBe(
      "error: https://e.com/x returned HTTP 404",
    );
    expect(
      await fetchPage({ url: "https://e.com/x" }, signal, ok("Blocked: abuse\nmore", 403)),
    ).toBe("error: https://e.com/x returned HTTP 403: Blocked: abuse");
    const down = vi.fn(async (_input: string, _init: RequestInit): Promise<Response> => {
      throw new Error("offline");
    });
    expect(await fetchPage({ url: "https://e.com/x" }, signal, down)).toBe(
      "error: could not reach https://e.com/x: offline",
    );
  });
});

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

describe("Agent with fetch", () => {
  const SRC = 'diagram "t" size (300,200)\n';
  it("offers fetch_url only when enabled and reports it as untrusted text", async () => {
    const off = scripted([
      [
        { type: "text_delta", text: "hi" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const a = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: off,
      model: "m",
      budget: 5,
    });
    for await (const _ of a.run("x")) void _;
    expect(off.requests[0]!.tools.map((t) => t.name)).not.toContain("fetch_url");
    expect((off.requests[0]!.messages[0] as { content: string }).content).not.toContain(
      "# Reading pages",
    );

    const on = scripted([
      [
        {
          type: "tool_call",
          call: { id: "c1", name: "fetch_url", arguments: JSON.stringify({ url: "nope" }) },
        },
        { type: "finish", reason: "tool_calls" },
      ],
      [
        { type: "text_delta", text: "done" },
        { type: "finish", reason: "stop" },
      ],
    ]);
    const b = new Agent({
      engine,
      doc: new Document(engine, SRC),
      provider: on,
      model: "m",
      budget: 5,
      fetch: true,
    });
    const events = [];
    for await (const e of b.run("read it")) events.push(e);
    expect(on.requests[0]!.tools.map((t) => t.name)).toContain(FETCH_TOOL.name);
    expect((on.requests[0]!.messages[0] as { content: string }).content).toContain(
      "# Reading pages",
    );
    const result = events.find((e) => e.type === "tool_result");
    expect(result).toMatchObject({ result: "error: `nope` is not a URL" });
  });
});
