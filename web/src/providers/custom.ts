import { streamChat } from "./chat.js";
import type { ChatEvent, ChatRequest, ModelRow, Provider } from "./types.js";

export interface CustomConfig {
  /** Base URL of an OpenAI-compatible API, with or without a trailing slash. */
  url: string;
  token: string;
  model: string;
}

export class CustomProvider implements Provider {
  readonly id = "openai-compatible";
  private readonly url: string;
  constructor(
    private readonly config: CustomConfig,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {
    this.url = config.url.replace(/\/+$/, "");
  }

  private headers(): Record<string, string> {
    return this.config.token === "" ? {} : { authorization: `Bearer ${this.config.token}` };
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
