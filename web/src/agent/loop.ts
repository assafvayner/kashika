import type { Document } from "../document/store.svelte.js";
import type { Engine } from "../engine/types.js";
import type { Message, Provider, ToolCall } from "../providers/types.js";
import { systemPrompt } from "./prompt.js";
import { TOOL_DEFS, runTool } from "./tools.js";

export type AgentEvent =
  | { type: "text"; text: string }
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

  constructor(private readonly opts: AgentOptions) {}

  stop(): void {
    this.controller?.abort();
  }

  async *run(userText: string): AsyncGenerator<AgentEvent> {
    const { engine, doc, provider, model, budget } = this.opts;
    this.controller = new AbortController();
    const signal = this.controller.signal;
    const context = [
      userText,
      "",
      doc.selection === null ? "" : `selected: ${doc.selection}`,
      "Current source:",
      "```kash",
      doc.source,
      "```",
    ]
      .filter((l, i) => i !== 2 || l !== "")
      .join("\n");
    this.history.push({ role: "user", content: userText });
    const turn: Message[] = [
      { role: "system", content: systemPrompt(engine) },
      ...this.history.slice(0, -1),
      { role: "user", content: context },
    ];
    const steps: string[] = [];
    let nudged = false;
    let finalText = "";
    try {
      for (;;) {
        let text = "";
        const calls: ToolCall[] = [];
        for await (const ev of provider.chat(
          { model, messages: [...turn], tools: TOOL_DEFS },
          signal,
        )) {
          if (ev.type === "text_delta") {
            text += ev.text;
            yield { type: "text", text: ev.text };
          } else if (ev.type === "tool_call") {
            calls.push(ev.call);
          }
        }
        if (text === "" && calls.length === 0) {
          if (nudged) {
            yield {
              type: "error",
              message: "The model returned nothing twice. Try another model.",
            };
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
        finalText += text;
        if (calls.length === 0) break;
        for (const call of calls) {
          if (steps.length >= budget) {
            this.finish(finalText, steps);
            yield { type: "stopped", steps: steps.length };
            return;
          }
          yield { type: "tool_start", call };
          let args: Record<string, unknown> = {};
          try {
            args = JSON.parse(call.arguments === "" ? "{}" : call.arguments) as Record<
              string,
              unknown
            >;
          } catch {
            args = {};
          }
          const result = runTool(engine, doc, call.name, args);
          const summary = summarize(call.name, call.arguments, result);
          steps.push(summary);
          turn.push({ role: "tool", tool_call_id: call.id, content: result });
          yield { type: "tool_result", call, result, summary };
        }
      }
      this.finish(finalText, steps);
      yield { type: "done" };
    } catch (e) {
      this.finish(finalText, steps);
      if (signal.aborted) {
        yield { type: "stopped", steps: steps.length };
      } else {
        yield { type: "error", message: e instanceof Error ? e.message : String(e) };
      }
    } finally {
      this.controller = null;
    }
  }

  private finish(text: string, steps: string[]): void {
    const content = [...steps, text].filter((s) => s !== "").join("\n");
    if (content !== "") this.history.push({ role: "assistant", content });
  }
}
