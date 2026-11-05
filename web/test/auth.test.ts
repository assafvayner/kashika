import { afterEach, describe, expect, it, vi } from "vitest";

const hub = vi.hoisted(() => ({
  redirect: vi.fn(),
  loginUrl: vi.fn(async () => "https://huggingface.co/oauth/authorize?x=1"),
}));
vi.mock("@huggingface/hub", () => ({
  oauthHandleRedirectIfPresent: hub.redirect,
  oauthLoginUrl: hub.loginUrl,
}));

import { completeLogin, loadSession } from "../src/auth.js";

describe("completeLogin", () => {
  afterEach(() => sessionStorage.clear());

  it("stores the session from a redirect and cleans the URL", async () => {
    hub.redirect.mockResolvedValueOnce({
      accessToken: "tok",
      accessTokenExpiresAt: new Date(Date.now() + 3600_000),
      userInfo: { preferred_username: "ann", name: "Ann" },
    });
    const replace = vi.spyOn(history, "replaceState");
    const s = await completeLogin();
    expect(s).toMatchObject({ token: "tok", name: "ann" });
    expect(loadSession()?.token).toBe("tok");
    expect(replace).toHaveBeenCalled();
  });

  it("returns the stored session when the URL carries no redirect", async () => {
    hub.redirect.mockResolvedValueOnce(false);
    sessionStorage.setItem(
      "kashika.hf",
      JSON.stringify({ token: "old", expires: Date.now() + 1000, name: "n" }),
    );
    expect((await completeLogin())?.token).toBe("old");
  });

  it("drops the redirect and keeps going when the exchange fails", async () => {
    hub.redirect.mockRejectedValueOnce(new Error("Missing oauth nonce"));
    expect(await completeLogin()).toBeNull();
  });
});
