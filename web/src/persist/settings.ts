import type { CustomConfig } from "../providers/custom.js";

export type Reasoning = "default" | "low" | "medium" | "high";
export const REASONING: Reasoning[] = ["default", "low", "medium", "high"];

export interface Settings {
  provider: "huggingface" | "openai-compatible";
  /** Hub model id for the Hugging Face provider; null until the first list loads. */
  hfModel: string | null;
  custom: CustomConfig;
  budget: number;
  /** Sent as `reasoning_effort`; "default" leaves it to the model. */
  reasoning: Reasoning;
}

export const DEFAULT_SETTINGS: Settings = {
  provider: "huggingface",
  hfModel: null,
  custom: { url: "", token: "", model: "" },
  budget: 30,
  reasoning: "low",
};

const KEY = "kashika.settings";

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return { ...DEFAULT_SETTINGS };
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null) return { ...DEFAULT_SETTINGS };
    const o = v as Record<string, unknown>;
    const c =
      typeof o.custom === "object" && o.custom !== null
        ? (o.custom as Record<string, unknown>)
        : {};
    const str = (x: unknown, d: string) => (typeof x === "string" ? x : d);
    const d = DEFAULT_SETTINGS;
    return {
      provider:
        o.provider === "huggingface" || o.provider === "openai-compatible"
          ? o.provider
          : d.provider,
      hfModel: typeof o.hfModel === "string" || o.hfModel === null ? o.hfModel : d.hfModel,
      custom: {
        url: str(c.url, d.custom.url),
        token: str(c.token, d.custom.token),
        model: str(c.model, d.custom.model),
      },
      budget:
        typeof o.budget === "number" && Number.isFinite(o.budget) && o.budget >= 1
          ? o.budget
          : d.budget,
      reasoning: (REASONING as string[]).includes(o.reasoning as string)
        ? (o.reasoning as Reasoning)
        : d.reasoning,
    };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(s: Settings): void {
  localStorage.setItem(KEY, JSON.stringify(s));
}
