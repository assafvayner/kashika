import { afterEach, describe, expect, it, vi } from "vitest";

const hub = vi.hoisted(() => ({
  redirect: vi.fn(),
  loginUrl: vi.fn(async () => "https://huggingface.co/oauth/authorize?x=1"),
}));
vi.mock("@huggingface/hub", () => ({
  oauthHandleRedirectIfPresent: hub.redirect,
  oauthLoginUrl: hub.loginUrl,
}));

import { completeLogin, listenForLogin, loadSession } from "../src/auth.js";

const result = {
  accessToken: "tok",
  accessTokenExpiresAt: new Date(Date.now() + 3600_000),
  userInfo: { preferred_username: "ann", name: "Ann" },
};

describe("login popup", () => {
  afterEach(() => {
    sessionStorage.clear();
    window.name = "";
    Object.defineProperty(window, "opener", { value: null, configurable: true });
    vi.restoreAllMocks();
  });

  it("hands the session to the opener and closes when it ran in the login popup", async () => {
    hub.redirect.mockResolvedValueOnce(result);
    const post = vi.fn();
    window.name = "kashika-login";
    Object.defineProperty(window, "opener", { value: { postMessage: post }, configurable: true });
    const close = vi.spyOn(window, "close").mockImplementation(() => {});
    const s = await completeLogin();
    expect(s?.token).toBe("tok");
    expect(post).toHaveBeenCalledWith({ type: "kashika-login", session: s }, location.origin);
    expect(close).toHaveBeenCalled();
  });

  it("stays put when it is the main window", async () => {
    hub.redirect.mockResolvedValueOnce(result);
    const close = vi.spyOn(window, "close").mockImplementation(() => {});
    expect((await completeLogin())?.name).toBe("ann");
    expect(close).not.toHaveBeenCalled();
    expect(loadSession()?.token).toBe("tok");
  });

  it("accepts a session only from its own origin and shape", () => {
    const cb = vi.fn();
    const stop = listenForLogin(cb);
    const session = { token: "t", expires: Date.now() + 1000, name: "n" };
    window.dispatchEvent(
      new MessageEvent("message", {
        origin: "https://evil.example",
        data: { type: "kashika-login", session },
      }),
    );
    window.dispatchEvent(
      new MessageEvent("message", {
        origin: location.origin,
        data: { type: "kashika-login", session: { token: 1 } },
      }),
    );
    expect(cb).not.toHaveBeenCalled();
    window.dispatchEvent(
      new MessageEvent("message", {
        origin: location.origin,
        data: { type: "kashika-login", session },
      }),
    );
    expect(cb).toHaveBeenCalledWith(session);
    expect(loadSession()?.token).toBe("t");
    stop();
    window.dispatchEvent(
      new MessageEvent("message", {
        origin: location.origin,
        data: { type: "kashika-login", session },
      }),
    );
    expect(cb).toHaveBeenCalledTimes(1);
  });
});
