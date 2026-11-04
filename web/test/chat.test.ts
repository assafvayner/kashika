import { describe, expect, it } from "vitest";

import { streamChat } from "../src/providers/chat.js";
import { parseSse } from "../src/providers/sse.js";
import { ProviderError } from "../src/providers/types.js";

function sse(chunks: string[], status = 200): typeof fetch {
  return async () =>
    new Response(
      new ReadableStream({
        start(c) {
          for (const ch of chunks) c.enqueue(new TextEncoder().encode(ch));
          c.close();
        },
      }),
      { status, headers: { "content-type": "text/event-stream" } },
    );
}

const delta = (d: object, finish: string | null = null) =>
  `data: ${JSON.stringify({ choices: [{ delta: d, finish_reason: finish }] })}\n\n`;

describe("parseSse", () => {
  it("joins split chunks and skips [DONE]", async () => {
    const stream = new ReadableStream({
      start(c) {
        c.enqueue(new TextEncoder().encode('data: {"a":1}\n\nda'));
        c.enqueue(new TextEncoder().encode('ta: {"a":2}\n\ndata: [DONE]\n\n'));
        c.close();
      },
    });
    const out: unknown[] = [];
    for await (const v of parseSse(stream)) out.push(v);
    expect(out).toEqual([{ a: 1 }, { a: 2 }]);
  });

  it("handles CRLF line endings", async () => {
    const stream = new ReadableStream({
      start(c) {
        c.enqueue(new TextEncoder().encode('data: {"a":1}\r\n\r\ndata: {"a":2}\r\n\r\n'));
        c.close();
      },
    });
    const out: unknown[] = [];
    for await (const v of parseSse(stream)) out.push(v);
    expect(out).toEqual([{ a: 1 }, { a: 2 }]);
  });
});

describe("streamChat", () => {
  const base = { url: "https://x/v1", headers: {} };
  const req = { model: "m", messages: [{ role: "user" as const, content: "hi" }], tools: [] };

  it("yields text and accumulates tool call arguments", async () => {
    const fetchImpl = sse([
      delta({ content: "Hel" }),
      delta({ content: "lo" }),
      delta({
        tool_calls: [
          { index: 0, id: "c1", function: { name: "write_source", arguments: '{"sou' } },
        ],
      }),
      delta({ tool_calls: [{ index: 0, function: { arguments: 'rce":"x"}' } }] }, "tool_calls"),
      "data: [DONE]\n\n",
    ]);
    const events = [];
    for await (const e of streamChat(base, req, new AbortController().signal, fetchImpl))
      events.push(e);
    expect(events).toEqual([
      { type: "text_delta", text: "Hel" },
      { type: "text_delta", text: "lo" },
      { type: "tool_call", call: { id: "c1", name: "write_source", arguments: '{"source":"x"}' } },
      { type: "finish", reason: "tool_calls" },
    ]);
  });

  it("throws ProviderError with the status", async () => {
    const fetchImpl: typeof fetch = async () => new Response("nope", { status: 401 });
    const it = streamChat(base, req, new AbortController().signal, fetchImpl)[
      Symbol.asyncIterator
    ]();
    await expect(it.next()).rejects.toMatchObject({ status: 401 });
    await expect(it.next().catch((e) => e)).resolves.toBeDefined();
    expect(new ProviderError("x", 429).status).toBe(429);
  });

  it("sends the OpenAI body", async () => {
    let body: unknown;
    const fetchImpl: typeof fetch = async (_u, init) => {
      body = JSON.parse(init!.body as string);
      return sse([delta({ content: "ok" }, "stop")])(_u, init);
    };
    const messages = [
      ...req.messages,
      {
        role: "assistant" as const,
        content: null,
        tool_calls: [{ id: "c1", name: "write_source", arguments: '{"source":"x"}' }],
      },
      { role: "tool" as const, tool_call_id: "c1", content: "ok" },
    ];
    for await (const _ of streamChat(
      base,
      {
        ...req,
        messages,
        tools: [{ name: "t", description: "d", parameters: { type: "object" } }],
      },
      new AbortController().signal,
      fetchImpl,
    )) {
      void _;
    }
    expect(body).toMatchObject({
      model: "m",
      stream: true,
      tools: [{ type: "function", function: { name: "t", description: "d" } }],
    });
    const sent = (body as { messages: unknown[] }).messages;
    expect(sent[1]).toEqual({
      role: "assistant",
      content: null,
      tool_calls: [
        {
          id: "c1",
          type: "function",
          function: { name: "write_source", arguments: '{"source":"x"}' },
        },
      ],
    });
    expect(sent[2]).toEqual({ role: "tool", tool_call_id: "c1", content: "ok" });
  });

  it("does not repeat a name sent in every delta", async () => {
    const fetchImpl = sse([
      delta({ tool_calls: [{ index: 0, id: "c1", function: { name: "w", arguments: "{" } }] }),
      delta({ tool_calls: [{ index: 0, function: { name: "w", arguments: "}" } }] }, "tool_calls"),
    ]);
    const events = [];
    for await (const e of streamChat(base, req, new AbortController().signal, fetchImpl))
      events.push(e);
    expect(events[0]).toEqual({
      type: "tool_call",
      call: { id: "c1", name: "w", arguments: "{}" },
    });
  });

  it("throws on an in-stream error chunk", async () => {
    const fetchImpl = sse([`data: ${JSON.stringify({ error: { message: "boom" } })}\n\n`]);
    const it = streamChat(base, req, new AbortController().signal, fetchImpl);
    await expect(it.next()).rejects.toThrow("boom");
  });

  it("throws ProviderError on malformed stream data", async () => {
    const fetchImpl = sse(["data: {oops\n\n"]);
    const it = streamChat(base, req, new AbortController().signal, fetchImpl);
    await expect(it.next()).rejects.toMatchObject({
      name: "Error",
      message: "malformed stream data: {oops",
      status: null,
    });
  });

  it("processes a trailing line without a newline", async () => {
    const stream = new ReadableStream({
      start(c) {
        c.enqueue(new TextEncoder().encode('data: {"a":1}'));
        c.close();
      },
    });
    const out: unknown[] = [];
    for await (const v of parseSse(stream)) out.push(v);
    expect(out).toEqual([{ a: 1 }]);
  });
});
