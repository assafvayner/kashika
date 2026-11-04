export interface ToolCall {
  id: string;
  name: string;
  /** JSON text of the arguments, as the model produced it. */
  arguments: string;
}

export type Message =
  | { role: "system"; content: string }
  | { role: "user"; content: string }
  | { role: "assistant"; content: string | null; tool_calls?: ToolCall[] }
  | { role: "tool"; tool_call_id: string; content: string };

export interface ToolDef {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface ChatRequest {
  model: string;
  messages: Message[];
  tools: ToolDef[];
}

export type ChatEvent =
  | { type: "text_delta"; text: string }
  | { type: "tool_call"; call: ToolCall }
  | { type: "finish"; reason: string | null };

export interface ModelRow {
  id: string;
  label: string;
  provider?: string;
}

export interface Provider {
  readonly id: "huggingface" | "openai-compatible";
  listModels(): Promise<ModelRow[]>;
  chat(req: ChatRequest, signal: AbortSignal): AsyncIterable<ChatEvent>;
}

export class ProviderError extends Error {
  constructor(
    message: string,
    public readonly status: number | null,
  ) {
    super(message);
  }
}
