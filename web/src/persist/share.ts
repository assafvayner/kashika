const PREFIX = "k=";

const MAX_DECODED = 1024 * 1024;

async function pipe(
  bytes: Uint8Array<ArrayBuffer>,
  stream: CompressionStream | DecompressionStream,
  max = Infinity,
): Promise<Uint8Array<ArrayBuffer>> {
  const reader = new Response(bytes).body!.pipeThrough(stream).getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > max) {
      await reader.cancel();
      throw new Error("too large");
    }
    chunks.push(value);
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

function toBase64Url(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(s: string): Uint8Array<ArrayBuffer> {
  const b64 = s.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (s.length % 4)) % 4);
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

/** The URL fragment (without `#`) that carries `source`: deflate-raw, base64url. */
export async function encodeShare(source: string): Promise<string> {
  const packed = await pipe(new TextEncoder().encode(source), new CompressionStream("deflate-raw"));
  return PREFIX + toBase64Url(packed);
}

/** The source in a fragment made by `encodeShare`, or null. */
export async function decodeShare(fragment: string): Promise<string | null> {
  if (!fragment.startsWith(PREFIX)) return null;
  try {
    const bytes = await pipe(
      fromBase64Url(fragment.slice(PREFIX.length)),
      new DecompressionStream("deflate-raw"),
      MAX_DECODED,
    );
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}
