import { describe, expect, it } from "vitest";

import { CustomProvider } from "../src/providers/custom.js";
import { HuggingFaceProvider, rankModels } from "../src/providers/huggingface.js";

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
  it("orders unranked models alphabetically", () => {
    const rows = rankModels(router, []);
    expect(rows.map((r) => r.id)).toEqual(["org/a", "org/b", "org/c"]);
  });
});

describe("HuggingFaceProvider", () => {
  it("ranks by the Hub trending list", async () => {
    const fetchImpl: typeof fetch = async (url) => {
      const u = String(url);
      if (u.startsWith("https://router.huggingface.co/v1/models")) return Response.json(router);
      expect(u).toContain("inference_provider=all");
      expect(u).toContain("sort=trendingScore");
      return Response.json([{ id: "org/c" }]);
    };
    const p = new HuggingFaceProvider(() => "tok", fetchImpl);
    expect((await p.listModels()).map((r) => r.id)).toEqual(["org/c", "org/a", "org/b"]);
  });
  it("tolerates a failing trending call", async () => {
    const fetchImpl: typeof fetch = async (url) => {
      if (String(url).startsWith("https://router.huggingface.co/v1/models"))
        return Response.json(router);
      return new Response("down", { status: 500 });
    };
    const p = new HuggingFaceProvider(() => "tok", fetchImpl);
    expect((await p.listModels()).map((r) => r.id)).toEqual(["org/a", "org/b", "org/c"]);
  });
  it("rejects chat on first next() with 401 when logged out", async () => {
    const p = new HuggingFaceProvider(
      () => null,
      async () => new Response("x"),
    );
    const it = p
      .chat({ model: "m", messages: [], tools: [] }, new AbortController().signal)
      [Symbol.asyncIterator]();
    await expect(it.next()).rejects.toMatchObject({ status: 401 });
  });
  it("sends the bearer token to the router", async () => {
    let auth = "";
    const fetchImpl: typeof fetch = async (_u, init) => {
      auth = new Headers(init!.headers).get("authorization")!;
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
  it("normalizes a pasted chat completions URL and sends the trimmed token", async () => {
    let seen = "";
    let auth: string | null = null;
    const f: typeof fetch = async (url, init) => {
      seen = String(url);
      auth = new Headers(init!.headers).get("authorization");
      return Response.json({ data: [{ id: "x" }] });
    };
    await new CustomProvider(
      { url: "https://e/v1/chat/completions/", token: " t ", model: "" },
      f,
    ).listModels();
    expect(seen).toBe("https://e/v1/models");
    expect(auth).toBe("Bearer t");
  });
});

describe("default fetch", () => {
  it("is called unbound", async () => {
    const original = globalThis.fetch;
    globalThis.fetch = function (this: unknown) {
      if (this !== undefined && this !== globalThis) throw new TypeError("Illegal invocation");
      return Promise.resolve(Response.json({ data: [{ id: "x" }] }));
    } as typeof fetch;
    try {
      await expect(new HuggingFaceProvider(() => "t").listModels()).resolves.toBeDefined();
      const rows = await new CustomProvider({
        url: "https://e/v1",
        token: "",
        model: "m",
      }).listModels();
      expect(rows.map((r) => r.id)).toEqual(["x"]);
    } finally {
      globalThis.fetch = original;
    }
  });
});
