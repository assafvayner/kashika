import { streamChat } from "./chat.js";
import type { ChatEvent, ChatRequest, ModelRow, Provider } from "./types.js";

export interface CustomConfig {
  /** Base URL including the API prefix, e.g. https://api.example.com/v1. */
  url: string;
  token: string;
  model: string;
}

export class CustomProvider implements Provider {
  readonly id = "openai-compatible";
  private readonly url: string;
  private readonly token: string;
  constructor(
    private readonly config: CustomConfig,
    private readonly fetchImpl: typeof fetch = (...args) => fetch(...args),
  ) {
    this.url = config.url
      .trim()
      .replace(/\/+$/, "")
      .replace(/\/chat\/completions$/, "");
    this.token = config.token.trim();
  }

  private headers(): Record<string, string> {
    return this.token === "" ? {} : { authorization: `Bearer ${this.token}` };
  }

  async listModels(): Promise<ModelRow[]> {
    const fallback =
      this.config.model === "" ? [] : [{ id: this.config.model, label: this.config.model }];
    try {
      const res = await this.fetchImpl(`${this.url}/models`, { headers: this.headers() });
      if (!res.ok) return fallback;
      const { data } = (await res.json()) as { data?: { id: string }[] };
      const rows = (data ?? []).map((m) => ({ id: m.id, label: m.id }));
      return rows.length > 0 ? rows : fallback;
    } catch {
      return fallback;
    }
  }

  chat(req: ChatRequest, signal: AbortSignal): AsyncIterable<ChatEvent> {
    return streamChat({ url: this.url, headers: this.headers() }, req, signal, this.fetchImpl);
  }
}
