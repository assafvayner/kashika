import { parseSse } from "./sse.js";
import { type ChatEvent, type ChatRequest, ProviderError, type ToolCall } from "./types.js";

export interface Endpoint {
  /** Base URL ending in `/v1`, no trailing slash. */
  url: string;
  headers: Record<string, string>;
}

interface Chunk {
  choices?: {
    delta?: {
      content?: string | null;
      tool_calls?: {
        index: number;
        id?: string;
        function?: { name?: string; arguments?: string };
      }[];
    };
    finish_reason?: string | null;
  }[];
  error?: { message?: string } | string;
}

/** One streaming chat completion in the OpenAI format, with tool calls assembled from deltas. */
export async function* streamChat(
  endpoint: Endpoint,
  req: ChatRequest,
  signal: AbortSignal,
  fetchImpl: typeof fetch = fetch,
): AsyncGenerator<ChatEvent> {
  const body = {
    model: req.model,
    messages: req.messages,
    stream: true,
    ...(req.tools.length > 0
      ? {
          tools: req.tools.map((t) => ({
            type: "function",
            function: { name: t.name, description: t.description, parameters: t.parameters },
          })),
          tool_choice: "auto",
        }
      : {}),
  };
  const res = await fetchImpl(`${endpoint.url}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json", ...endpoint.headers },
    body: JSON.stringify(body),
    signal,
  });
  if (!res.ok || res.body === null) {
    const text = await res.text().catch(() => "");
    throw new ProviderError(`${res.status} ${res.statusText}: ${text.slice(0, 500)}`, res.status);
  }
  const calls = new Map<number, ToolCall>();
  let finish: string | null = null;
  for await (const raw of parseSse(res.body)) {
    const chunk = raw as Chunk;
    if (chunk.error !== undefined) {
      const msg = typeof chunk.error === "string" ? chunk.error : (chunk.error.message ?? "error");
      throw new ProviderError(msg, null);
    }
    const choice = chunk.choices?.[0];
    if (choice === undefined) continue;
    const text = choice.delta?.content;
    if (typeof text === "string" && text !== "") yield { type: "text_delta", text };
    for (const tc of choice.delta?.tool_calls ?? []) {
      const call = calls.get(tc.index) ?? { id: "", name: "", arguments: "" };
      if (tc.id) call.id = tc.id;
      if (tc.function?.name) call.name += tc.function.name;
      if (tc.function?.arguments) call.arguments += tc.function.arguments;
      calls.set(tc.index, call);
    }
    if (choice.finish_reason) finish = choice.finish_reason;
  }
  for (const [, call] of [...calls.entries()].sort((a, b) => a[0] - b[0])) {
    if (call.id === "") call.id = `call_${Math.random().toString(36).slice(2, 10)}`;
    yield { type: "tool_call", call };
  }
  yield { type: "finish", reason: finish };
}
