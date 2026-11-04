import { parseSse } from "./sse.js";
import {
  type ChatEvent,
  type ChatRequest,
  type Message,
  ProviderError,
  type ToolCall,
} from "./types.js";

export interface Endpoint {
  /** Base URL ending in `/v1`, no trailing slash. */
  url: string;
  headers: Record<string, string>;
}

interface Chunk {
  choices?: {
    delta?: {
      content?: string | null;
      reasoning_content?: string | null;
      reasoning?: string | null;
      tool_calls?: {
        index?: number;
        id?: string;
        function?: { name?: string; arguments?: string };
      }[];
    };
    finish_reason?: string | null;
  }[];
  error?: { message?: string } | string;
}

const toWire = (m: Message) =>
  m.role === "assistant" && m.tool_calls?.length
    ? {
        role: "assistant",
        content: m.content,
        tool_calls: m.tool_calls.map((c) => ({
          id: c.id,
          type: "function",
          function: { name: c.name, arguments: c.arguments },
        })),
      }
    : m;

function httpErrorMessage(status: number, statusText: string, text: string): string {
  try {
    const j = JSON.parse(text) as { error?: { message?: string } | string } | null;
    const m = typeof j?.error === "string" ? j.error : j?.error?.message;
    if (m) return `${status}: ${m}`;
  } catch {
    // not JSON, fall through to the excerpt
  }
  return `${status} ${statusText}: ${text.slice(0, 500)}`;
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
    messages: req.messages.map(toWire),
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
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new ProviderError(httpErrorMessage(res.status, res.statusText, text), res.status);
  }
  if (res.body === null) throw new ProviderError("empty response body", res.status);
  const calls: ToolCall[] = [];
  const current = new Map<number, ToolCall>();
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
    const reasoning = choice.delta?.reasoning_content ?? choice.delta?.reasoning;
    if (typeof reasoning === "string" && reasoning !== "")
      yield { type: "reasoning_delta", text: reasoning };
    for (const tc of choice.delta?.tool_calls ?? []) {
      const index = tc.index ?? 0;
      let call = current.get(index);
      if (call === undefined || (tc.id && call.id !== "" && tc.id !== call.id)) {
        call = { id: "", name: "", arguments: "" };
        current.set(index, call);
        calls.push(call);
      }
      if (tc.id) call.id = tc.id;
      if (tc.function?.name && call.name === "") call.name = tc.function.name;
      if (tc.function?.arguments) call.arguments += tc.function.arguments;
    }
    if (choice.finish_reason) finish = choice.finish_reason;
  }
  for (const call of calls) {
    if (call.id === "") call.id = `call_${Math.random().toString(36).slice(2, 10)}`;
    yield { type: "tool_call", call };
  }
  yield { type: "finish", reason: finish };
}
