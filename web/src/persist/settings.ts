import type { CustomConfig } from "../providers/custom.js";

export interface Settings {
  provider: "huggingface" | "openai-compatible";
  /** Hub model id for the Hugging Face provider; null until the first list loads. */
  hfModel: string | null;
  custom: CustomConfig;
  budget: number;
}

export const DEFAULT_SETTINGS: Settings = {
  provider: "huggingface",
  hfModel: null,
  custom: { url: "", token: "", model: "" },
  budget: 20,
};

const KEY = "kashika.settings";

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return { ...DEFAULT_SETTINGS };
    const v = JSON.parse(raw) as Partial<Settings>;
    return {
      ...DEFAULT_SETTINGS,
      ...v,
      custom: { ...DEFAULT_SETTINGS.custom, ...(v.custom ?? {}) },
    };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(s: Settings): void {
  localStorage.setItem(KEY, JSON.stringify(s));
}
