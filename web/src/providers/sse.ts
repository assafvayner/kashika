import { ProviderError } from "./types.js";

function parseLine(raw: string): { done: boolean; value?: unknown } {
  const line = raw.replace(/\r$/, "");
  if (!line.startsWith("data:")) return { done: false };
  const data = line.slice(5).trim();
  if (data === "[DONE]") return { done: true };
  if (data === "") return { done: false };
  try {
    return { done: false, value: JSON.parse(data) };
  } catch {
    throw new ProviderError(`malformed stream data: ${data.slice(0, 200)}`, null);
  }
}

/**
 * Parses an SSE byte stream into the JSON of each `data:` line, stopping at `[DONE]`.
 * After `[DONE]` the rest of the body is drained so the connection closes normally;
 * the stream is cancelled only when the consumer stops early.
 */
export async function* parseSse(stream: ReadableStream<Uint8Array>): AsyncGenerator<unknown> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buf = "";
  let finished = false;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      let nl: number;
      while ((nl = buf.indexOf("\n")) >= 0) {
        const line = buf.slice(0, nl);
        buf = buf.slice(nl + 1);
        const r = parseLine(line);
        if (r.done) {
          while (!(await reader.read()).done);
          finished = true;
          return;
        }
        if (r.value !== undefined) yield r.value;
      }
    }
    const r = parseLine(buf + decoder.decode());
    if (!r.done && r.value !== undefined) yield r.value;
    finished = true;
  } finally {
    if (!finished) await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}
