import type { Document } from "../document/store.svelte.js";
import type { Engine } from "../engine/types.js";
import type { Message, Provider, ToolCall } from "../providers/types.js";
import { systemPrompt } from "./prompt.js";
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
}

const NUDGE = "Your reply was empty. Either call a tool or answer the user in a sentence.";

function summarize(name: string, args: string, result: string): string {
  const first = result.split("\n").find((l) => l.trim() !== "") ?? "";
  const m = /(\d+) errors?, (\d+) warnings?/.exec(result);
  const tail = m ? m[0] : first.slice(0, 80);
  return `[${name}${name === "write_source" ? "" : ` ${args.slice(0, 60)}`}: ${tail}]`;
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
    const { engine, doc, provider, model, budget } = this.opts;
    this.controller = new AbortController();
    const signal = this.controller.signal;
    this.system ??= systemPrompt(engine);
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
          { model, messages: [...turn], tools: TOOL_DEFS },
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
            result = runTool(engine, doc, call.name, args);
            summary = summarize(call.name, call.arguments, result);
          }
          steps.push(summary);
          turn.push({ role: "tool", tool_call_id: call.id, content: result });
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
