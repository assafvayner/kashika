import { oauthHandleRedirectIfPresent, oauthLoginUrl } from "@huggingface/hub";

const KEY = "kashika.hf";

export interface Session {
  token: string;
  expires: number;
  name: string;
}

export function loadSession(): Session | null {
  try {
    const raw = sessionStorage.getItem(KEY);
    if (raw === null) return null;
    const s = JSON.parse(raw) as Session;
    return s.expires > Date.now() ? s : null;
  } catch {
    return null;
  }
}

export function clearSession(): void {
  sessionStorage.removeItem(KEY);
}

export function storeSession(s: Session): void {
  sessionStorage.setItem(KEY, JSON.stringify(s));
}

/** The window name of the login popup, so the redirect knows to hand its session back. */
const LOGIN_WINDOW = "kashika-login";
const LOGIN_MESSAGE = "kashika-login";

function isSession(v: unknown): v is Session {
  const s = v as Session | null;
  return (
    typeof s === "object" &&
    s !== null &&
    typeof s.token === "string" &&
    typeof s.expires === "number" &&
    typeof s.name === "string"
  );
}

/** Completes a login redirect if the URL carries one, storing the session and cleaning the URL. */
export async function completeLogin(): Promise<Session | null> {
  let r;
  try {
    r = await oauthHandleRedirectIfPresent();
  } catch {
    history.replaceState(null, "", location.pathname + location.hash);
    return loadSession();
  }
  if (r === false) return loadSession();
  const s: Session = {
    token: r.accessToken,
    expires: r.accessTokenExpiresAt.getTime(),
    name: r.userInfo.preferred_username || r.userInfo.name,
  };
  storeSession(s);
  history.replaceState(null, "", location.pathname + location.hash);
  if (window.name === LOGIN_WINDOW && window.opener) {
    (window.opener as Window).postMessage({ type: LOGIN_MESSAGE, session: s }, location.origin);
    window.close();
  }
  return s;
}

/** Calls `cb` with the session a login popup hands back; returns the unsubscribe. */
export function listenForLogin(cb: (s: Session) => void): () => void {
  const onMessage = (e: MessageEvent) => {
    if (e.origin !== location.origin) return;
    const d = e.data as { type?: unknown; session?: unknown } | null;
    if (d?.type !== LOGIN_MESSAGE || !isSession(d.session)) return;
    storeSession(d.session);
    cb(d.session);
  };
  window.addEventListener("message", onMessage);
  return () => window.removeEventListener("message", onMessage);
}

function loginUrl(): Promise<string> {
  const clientId = import.meta.env.DEV
    ? (import.meta.env["VITE_OAUTH_CLIENT_ID"] as string | undefined)
    : undefined;
  const redirectUrl =
    (import.meta.env.DEV
      ? (import.meta.env["VITE_OAUTH_REDIRECT_URL"] as string | undefined)
      : undefined) || location.origin + location.pathname;
  return oauthLoginUrl({
    clientId: clientId || undefined,
    redirectUrl,
    scopes: clientId ? "openid profile inference-api" : undefined,
  });
}

let preparedUrl: string | null = null;

/** Fetches the login URL ahead of the click, so an embedded app can open the popup synchronously. */
export function prepareLogin(): void {
  if (window.self === window.top || preparedUrl !== null) return;
  void loginUrl().then(
    (u) => (preparedUrl = u),
    () => {},
  );
}

/**
 * Starts the login. In a Space the client id and scopes come from the Space config; locally from env.
 * Embedded in the Space page, the login runs in a popup that hands the session back and closes.
 */
export async function login(): Promise<void> {
  if (window.self === window.top) {
    location.href = await loginUrl();
    return;
  }
  const url = preparedUrl;
  preparedUrl = null;
  if (url !== null) {
    window.open(url, LOGIN_WINDOW);
    return;
  }
  const popup = window.open("", LOGIN_WINDOW);
  const fresh = await loginUrl();
  if (popup) popup.location.href = fresh;
  else location.href = fresh;
}
