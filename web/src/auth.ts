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
  sessionStorage.setItem(KEY, JSON.stringify(s));
  history.replaceState(null, "", location.pathname + location.hash);
  return s;
}

/** Starts the login. In a Space the client id and scopes come from the Space config; locally from env. */
export async function login(): Promise<void> {
  const popup = window.self !== window.top ? window.open("", "_blank") : null;
  const clientId = import.meta.env.DEV
    ? (import.meta.env["VITE_OAUTH_CLIENT_ID"] as string | undefined)
    : undefined;
  const redirectUrl =
    (import.meta.env.DEV
      ? (import.meta.env["VITE_OAUTH_REDIRECT_URL"] as string | undefined)
      : undefined) || location.origin + location.pathname;
  const url = await oauthLoginUrl({
    clientId: clientId || undefined,
    redirectUrl,
    scopes: clientId ? "openid profile inference-api" : undefined,
  });
  if (popup) popup.location.href = url;
  else location.href = url;
}
