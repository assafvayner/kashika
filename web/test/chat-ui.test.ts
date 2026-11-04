import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { ChatEntry } from "../src/persist/projects.js";
import type { AppState } from "../src/state.svelte.js";
import Chat from "../src/ui/Chat.svelte";
import ModelPicker from "../src/ui/ModelPicker.svelte";

describe("Chat", () => {
  it("renders messages and steps, sends on Enter, shows the selection chip", async () => {
    const chat: ChatEntry[] = [
      { role: "user", text: "hi" },
      {
        role: "assistant",
        text: "Added B.",
        steps: [{ summary: "[write_source: clean]", result: "diagram.kash: clean" }],
      },
    ];
    const onsend = vi.fn();
    const onclear = vi.fn();
    render(Chat, {
      chat,
      running: false,
      ready: true,
      selection: "db",
      onsend,
      onstop: () => {},
      onclear,
    });
    expect(screen.getByText("Added B.")).toBeTruthy();
    expect(screen.getByText("[write_source: clean]")).toBeTruthy();
    expect(screen.getByText("db")).toBeTruthy();
    const input = screen.getByRole("textbox") as HTMLTextAreaElement;
    await fireEvent.input(input, { target: { value: "move this" } });
    await fireEvent.keyDown(input, { key: "Enter" });
    expect(onsend).toHaveBeenCalledWith("move this");
  });

  it("prompts to log in when not ready", () => {
    render(Chat, {
      chat: [],
      running: false,
      ready: false,
      selection: null,
      onsend: () => {},
      onstop: () => {},
      onclear: () => {},
    });
    expect(screen.getByText(/log in or add an endpoint/i)).toBeTruthy();
  });
});

describe("Chat controls", () => {
  const base = { ready: true, selection: null, onsend: () => {}, onclear: () => {} };

  it("shows Stop instead of Send while running", async () => {
    const onstop = vi.fn();
    render(Chat, { ...base, chat: [], running: true, onstop });
    expect(screen.queryByText("Send")).toBeNull();
    await fireEvent.click(screen.getByText("Stop"));
    expect(onstop).toHaveBeenCalled();
  });

  it("toggles a step result", async () => {
    const chat: ChatEntry[] = [
      { role: "assistant", text: "ok", steps: [{ summary: "[step]", result: "the result" }] },
    ];
    render(Chat, { ...base, chat, running: false, onstop: () => {} });
    expect(screen.queryByText("the result")).toBeNull();
    await fireEvent.click(screen.getByText("[step]"));
    expect(screen.getByText("the result")).toBeTruthy();
    await fireEvent.click(screen.getByText("[step]"));
    expect(screen.queryByText("the result")).toBeNull();
  });

  it("clears the selection from the chip", async () => {
    const onclear = vi.fn();
    render(Chat, { ...base, chat: [], running: false, selection: "db", onstop: () => {}, onclear });
    await fireEvent.click(screen.getByTitle("Clear selection"));
    expect(onclear).toHaveBeenCalled();
  });
});

describe("ModelPicker", () => {
  const app = {
    session: null,
    settings: {
      provider: "huggingface",
      hfModel: null,
      custom: { url: "", token: "", model: "" },
      budget: 20,
    },
    models: [],
    modelsError: null,
  } as unknown as AppState;

  it("offers login on the Hugging Face tab", () => {
    render(ModelPicker, { app, onclose: () => {} });
    expect(screen.getByRole("button", { name: "Log in" })).toBeTruthy();
  });

  it("disables the custom endpoint button while the URL is empty", async () => {
    render(ModelPicker, { app, onclose: () => {} });
    await fireEvent.click(screen.getByText("Custom endpoint"));
    const btn = screen.getByText("Use this endpoint") as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });
});

describe("Chat retry", () => {
  it("shows Retry after an error and calls onretry", async () => {
    const chat: ChatEntry[] = [
      { role: "user", text: "draw" },
      { role: "assistant", text: "\n\nError: 500 upstream failed", steps: [] },
    ];
    const onretry = vi.fn();
    render(Chat, {
      chat,
      running: false,
      ready: true,
      selection: null,
      onsend: () => {},
      onstop: () => {},
      onclear: () => {},
      error: { message: "500 upstream failed", status: 500 },
      onretry,
    });
    await fireEvent.click(screen.getByText("Retry"));
    expect(onretry).toHaveBeenCalled();
  });
});
