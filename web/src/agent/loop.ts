import type { Document } from "../document/store.svelte.js";
import type { Engine } from "../engine/types.js";
import type { Message, Provider, ToolCall } from "../providers/types.js";
import { systemPrompt } from "./prompt.js";
import { FETCH_RULES, FETCH_TOOL, fetchPage } from "./fetch.js";
import { TOOL_DEFS, runTool } from "./tools.js";

export type AgentEvent =
  | { type: "text"; text: string }
  | { type: "thinking"; text: string }
  | { type: "model_start" }
  | { type: "tool_start"; call: ToolCall }
  | { type: "tool_result"; call: ToolCall; result: string; summary: string }
  | { type: "done" }
  | { type: "stopped"; steps: number }
  | { type: "error"; message: string };

export interface AgentOptions {
  engine: Engine;
  doc: Document;
  provider: Provider;
  model: string;
  /** Tool calls per turn. */
  budget: number;
  reasoning?: "low" | "medium" | "high";
  /** Offers fetch_url, which reads pages through a reader proxy. */
  fetch?: boolean;
}

const NUDGE = "Your reply was empty. Either call a tool or answer the user in a sentence.";

function summarize(name: string, args: string, result: string): string {
  const first = result.split("\n").find((l) => l.trim() !== "") ?? "";
  const m = /(\d+) errors?, (\d+) warnings?/.exec(result);
  const tail = m ? m[0] : first.slice(0, 80);
  const quiet = name === "write_source" || name === "edit_source";
  return `[${name}${quiet ? "" : ` ${args.slice(0, 60)}`}: ${tail}]`;
}

/** Tool results kept in full at the end of the turn; older ones collapse to their summary. */
const KEEP = 2;

/**
 * The turn as the model sees it: every tool result but the last few collapses to its summary
 * line, and the arguments of the old writes go too, so a long turn stays a few tens of KB
 * instead of carrying every report. The current source then rides on the last result, since
 * the one in the first user message is stale after the first write.
 */
export function compact(turn: Message[], source: string): Message[] {
  const toolAt = turn.flatMap((m, i) => (m.role === "tool" ? [i] : []));
  const old = new Set(toolAt.slice(0, Math.max(0, toolAt.length - KEEP)));
  if (old.size === 0) return turn.map(strip);
  const oldCalls = new Set<string>();
  const out = turn.map((m, i) => {
    if (m.role === "tool" && old.has(i)) {
      oldCalls.add(m.tool_call_id);
      return strip({ ...m, content: `${m.summary ?? m.content.split("\n")[0]} (report elided)` });
    }
    return strip(m);
  });
  for (const m of out) {
    if (m.role !== "assistant" || !m.tool_calls) continue;
    m.tool_calls = m.tool_calls.map((c) =>
      oldCalls.has(c.id) && (c.name === "write_source" || c.name === "edit_source")
        ? { ...c, arguments: `{"elided": "${c.arguments.length} chars"}` }
        : c,
    );
  }
  const last = toolAt.at(-1);
  if (last !== undefined) {
    const m = out[last]!;
    if (m.role === "tool") {
      m.content = `${m.content}\n\nCurrent source:\n\`\`\`kash\n${source}\n\`\`\``;
    }
  }
  return out;
}

function strip(m: Message): Message {
  if (m.role !== "tool") return { ...m };
  const { summary: _summary, ...rest } = m;
  return rest;
}

export class Agent {
  /** Chat history across turns: user and assistant messages, with tool activity collapsed. */
  history: Message[] = [];
  private controller: AbortController | null = null;
  private system: string | null = null;

  constructor(private readonly opts: AgentOptions) {}

  stop(): void {
    this.controller?.abort();
  }

  async *run(userText: string): AsyncGenerator<AgentEvent> {
    if (this.controller !== null) {
      yield { type: "error", message: "agent is busy" };
      return;
    }
    const { engine, doc, provider, model, budget, reasoning } = this.opts;
    this.controller = new AbortController();
    const signal = this.controller.signal;
    const tools = this.opts.fetch ? [...TOOL_DEFS, FETCH_TOOL] : TOOL_DEFS;
    this.system ??= systemPrompt(engine) + (this.opts.fetch ? FETCH_RULES : "");
    const context = [
      userText,
      "",
      ...(doc.selection === null ? [] : [`selected: ${doc.selection}`]),
      "Current source:",
      "```kash",
      doc.source,
      "```",
    ].join("\n");
    const turn: Message[] = [
      { role: "system", content: this.system },
      ...this.history,
      { role: "user", content: context },
    ];
    this.history.push({ role: "user", content: userText });
    const steps: string[] = [];
    const texts: string[] = [];
    let nudged = false;
    try {
      for (;;) {
        let text = "";
        const calls: ToolCall[] = [];
        yield { type: "model_start" };
        for await (const ev of provider.chat(
          { model, messages: compact(turn, doc.source), tools, reasoning },
          signal,
        )) {
          if (ev.type === "text_delta") {
            text += ev.text;
            yield { type: "text", text: ev.text };
          } else if (ev.type === "reasoning_delta") {
            yield { type: "thinking", text: ev.text };
          } else if (ev.type === "tool_call") {
            calls.push(ev.call);
          }
        }
        if (text === "" && calls.length === 0) {
          if (nudged) {
            const message = "The model returned nothing twice. Try another model.";
            this.finish(texts, steps, `[error: ${message}]`);
            yield { type: "error", message };
            return;
          }
          nudged = true;
          turn.push({ role: "user", content: NUDGE });
          continue;
        }
        turn.push({
          role: "assistant",
          content: text === "" ? null : text,
          tool_calls: calls.length ? calls : undefined,
        });
        if (text !== "") texts.push(text);
        if (calls.length === 0) break;
        for (const call of calls) {
          if (steps.length >= budget) {
            this.finish(texts, steps, `[stopped: step budget after ${steps.length} steps]`);
            yield { type: "stopped", steps: steps.length };
            return;
          }
          yield { type: "tool_start", call };
          let result = "";
          let summary = "";
          let args: Record<string, unknown> | null = null;
          try {
            args = JSON.parse(call.arguments === "" ? "{}" : call.arguments) as Record<
              string,
              unknown
            >;
          } catch (e) {
            result = `error: arguments are not valid JSON (${(e as Error).message}); send the call again with valid JSON`;
            summary = `[${call.name}: invalid arguments]`;
          }
          if (args !== null) {
            result =
              call.name === "fetch_url" && this.opts.fetch
                ? await fetchPage(args, signal)
                : runTool(engine, doc, call.name, args);
            summary = summarize(call.name, call.arguments, result);
          }
          steps.push(summary);
          turn.push({ role: "tool", tool_call_id: call.id, content: result, summary });
          yield { type: "tool_result", call, result, summary };
        }
      }
      this.finish(texts, steps);
      yield { type: "done" };
    } catch (e) {
      if (signal.aborted) {
        this.finish(texts, steps, "[stopped by user]");
        yield { type: "stopped", steps: steps.length };
      } else {
        const message = e instanceof Error ? e.message : String(e);
        this.finish(texts, steps, `[error: ${message}]`);
        yield { type: "error", message };
      }
    } finally {
      if (this.controller?.signal === signal) this.controller = null;
    }
  }

  private finish(texts: string[], steps: string[], note?: string): void {
    const content = [...steps, texts.join("\n"), ...(note ? [note] : [])]
      .filter((s) => s !== "")
      .join("\n");
    this.history.push({ role: "assistant", content });
  }
}
