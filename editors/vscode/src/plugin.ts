import type MarkdownIt from "markdown-it";

export interface Span {
  line: number;
  col: number;
}

export interface Diagnostic {
  severity: "error" | "warn";
  code: string;
  message: string;
  span: Span;
  suppressed?: boolean;
}

export interface Rendered {
  svg: string | null;
  errors: number;
  warnings: number;
  diagnostics: Diagnostic[];
}

export type Render = (src: string) => Rendered;

export const LANGUAGES = ["kashika", "kash"];

const CACHE_LIMIT = 64;

/** Caches `render` by source, so editing prose around a diagram doesn't re-render it. */
export function cached(render: Render): Render {
  const cache = new Map<string, Rendered>();
  return (src) => {
    let hit = cache.get(src);
    if (hit === undefined) {
      hit = render(src);
      if (cache.size >= CACHE_LIMIT) {
        cache.delete(cache.keys().next().value!);
      }
    } else {
      cache.delete(src);
    }
    cache.set(src, hit);
    return hit;
  };
}

/** Renders ```kashika and ```kash fences as inline SVG images, with their diagnostics below. */
export function kashikaPlugin(md: MarkdownIt, render: Render): void {
  const fallback = md.renderer.rules.fence!;
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const token = tokens[idx];
    const lang = token.info.trim().split(/\s+/)[0];
    if (!LANGUAGES.includes(lang)) {
      return fallback(tokens, idx, options, env, self);
    }
    const first = token.map ? token.map[0] : 0;
    let rendered: Rendered;
    try {
      rendered = render(token.content);
    } catch (e) {
      return block(first, "", failure(md, `kashika failed: ${e}`));
    }
    const img = rendered.svg
      ? `<img alt="kashika diagram" src="data:image/svg+xml;base64,${Buffer.from(rendered.svg).toString("base64")}">`
      : "";
    return block(first, img, diagnostics(md, rendered, first));
  };
}

function block(line: number, img: string, notes: string): string {
  return `<div class="kashika-diagram code-line" data-line="${line}">${img}${notes}</div>\n`;
}

function diagnostics(md: MarkdownIt, r: Rendered, fence: number): string {
  const shown = r.diagnostics.filter((d) => !d.suppressed);
  if (shown.length === 0) {
    return r.svg === null ? failure(md, "kashika: nothing to render (no diagram statement)") : "";
  }
  const items = shown.map((d) => {
    const line = fence + 1 + d.span.line;
    const text = `${line}:${d.span.col} ${d.severity}[${d.code}]: ${d.message}`;
    return `<div class="kashika-${d.severity}">${md.utils.escapeHtml(text)}</div>`;
  });
  return `<div class="kashika-diagnostics">${items.join("")}</div>`;
}

function failure(md: MarkdownIt, message: string): string {
  return `<div class="kashika-diagnostics"><div class="kashika-error">${md.utils.escapeHtml(message)}</div></div>`;
}
