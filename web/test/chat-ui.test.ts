import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { ChatEntry } from "../src/persist/projects.js";
import Chat from "../src/ui/Chat.svelte";

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
