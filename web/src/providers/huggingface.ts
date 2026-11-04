import { streamChat } from "./chat.js";
import type { ChatEvent, ChatRequest, ModelRow, Provider } from "./types.js";

export const ROUTER = "https://router.huggingface.co/v1";
export const HUB = "https://huggingface.co";
/** Used when the model list cannot be loaded. */
export const DEFAULT_MODEL = "deepseek-ai/DeepSeek-V4.1-Flash-1017";

export interface RouterModels {
  data: {
    id: string;
    providers?: { provider: string; status: string; supports_tools?: boolean }[];
  }[];
}

/** Router models with a live tool-capable provider, in Hub trending order; unranked ones follow alphabetically. */
export function rankModels(router: RouterModels, trending: { id: string }[]): ModelRow[] {
  const rank = new Map(trending.map((m, i) => [m.id, i]));
  const rows: (ModelRow & { rank: number })[] = [];
  for (const m of router.data) {
    const p = (m.providers ?? []).find((p) => p.status === "live" && p.supports_tools === true);
    if (p === undefined) continue;
    rows.push({ id: m.id, label: m.id, provider: p.provider, rank: rank.get(m.id) ?? Infinity });
  }
  rows.sort((a, b) => a.rank - b.rank || a.id.localeCompare(b.id));
  return rows.map(({ rank: _, ...r }) => r);
}

export class HuggingFaceProvider implements Provider {
  readonly id = "huggingface";
  constructor(
    private readonly token: () => string | null,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {}

  async listModels(): Promise<ModelRow[]> {
    const routerRes = await this.fetchImpl(`${ROUTER}/models`);
    if (!routerRes.ok) throw new Error(`router models: ${routerRes.status}`);
    const router = (await routerRes.json()) as RouterModels;
    let trending: { id: string }[] = [];
    try {
      const res = await this.fetchImpl(
        `${HUB}/api/models?inference_provider=all&filter=conversational&sort=trendingScore&limit=200`,
      );
      if (res.ok) trending = (await res.json()) as { id: string }[];
    } catch {
      trending = [];
    }
    return rankModels(router, trending);
  }

  chat(req: ChatRequest, signal: AbortSignal): AsyncIterable<ChatEvent> {
    const token = this.token();
    if (token === null) throw new Error("not logged in");
    return streamChat(
      { url: ROUTER, headers: { authorization: `Bearer ${token}` } },
      req,
      signal,
      this.fetchImpl,
    );
  }
}
