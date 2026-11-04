const PREFIX = "k=";

async function pipe(
  bytes: Uint8Array<ArrayBuffer>,
  stream: CompressionStream | DecompressionStream,
): Promise<Uint8Array<ArrayBuffer>> {
  const out = new Response(bytes).body!.pipeThrough(stream);
  return new Uint8Array(await new Response(out).arrayBuffer());
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
    );
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}
