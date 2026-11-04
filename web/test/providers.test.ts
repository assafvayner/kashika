import { describe, expect, it } from "vitest";

import { CustomProvider } from "../src/providers/custom.js";
import { DEFAULT_MODEL, HuggingFaceProvider, rankModels } from "../src/providers/huggingface.js";

const router = {
  data: [
    { id: "org/no-tools", providers: [{ provider: "p", status: "live", supports_tools: false }] },
    { id: "org/dead", providers: [{ provider: "p", status: "error", supports_tools: true }] },
    { id: "org/b", providers: [{ provider: "p", status: "live", supports_tools: true }] },
    {
      id: "org/a",
      providers: [
        { provider: "p", status: "live", supports_tools: true },
        { provider: "q", status: "live" },
      ],
    },
    { id: "org/c", providers: [{ provider: "p", status: "live", supports_tools: true }] },
  ],
};

describe("rankModels", () => {
  it("keeps tool-capable live models in trending order, then the rest alphabetically", () => {
    const rows = rankModels(router, [{ id: "org/c" }, { id: "org/no-tools" }, { id: "org/a" }]);
    expect(rows.map((r) => r.id)).toEqual(["org/c", "org/a", "org/b"]);
    expect(rows[0]!.provider).toBe("p");
  });
  it("falls back to the default when there is no trending list", () => {
    const rows = rankModels(router, []);
    expect(rows.map((r) => r.id)).toEqual(["org/a", "org/b", "org/c"]);
    expect(DEFAULT_MODEL).toBe("deepseek-ai/DeepSeek-V4.1-Flash-1017");
  });
});

describe("HuggingFaceProvider", () => {
  it("lists through both APIs and tolerates a failing trending call", async () => {
    const fetchImpl: typeof fetch = async (url) => {
      const u = String(url);
      if (u.startsWith("https://router.huggingface.co/v1/models")) return Response.json(router);
      return new Response("down", { status: 500 });
    };
    const p = new HuggingFaceProvider(() => "tok", fetchImpl);
    expect((await p.listModels()).map((r) => r.id)).toEqual(["org/a", "org/b", "org/c"]);
  });
  it("sends the bearer token to the router", async () => {
    let auth = "";
    const fetchImpl: typeof fetch = async (_u, init) => {
      auth = (init!.headers as Record<string, string>)["authorization"]!;
      return new Response("x", { status: 500 });
    };
    const p = new HuggingFaceProvider(() => "tok", fetchImpl);
    const it = p
      .chat({ model: "m", messages: [], tools: [] }, new AbortController().signal)
      [Symbol.asyncIterator]();
    await expect(it.next()).rejects.toMatchObject({ status: 500 });
    expect(auth).toBe("Bearer tok");
  });
});

describe("CustomProvider", () => {
  it("lists /models and falls back to the configured model", async () => {
    const ok: typeof fetch = async () => Response.json({ data: [{ id: "x" }, { id: "y" }] });
    expect(
      (
        await new CustomProvider({ url: "https://e/v1/", token: "", model: "z" }, ok).listModels()
      ).map((r) => r.id),
    ).toEqual(["x", "y"]);
    const bad: typeof fetch = async () => new Response("", { status: 404 });
    expect(
      (
        await new CustomProvider({ url: "https://e/v1", token: "", model: "z" }, bad).listModels()
      ).map((r) => r.id),
    ).toEqual(["z"]);
  });
});
