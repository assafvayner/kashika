import init, * as wasm from "./pkg/kashika_wasm.js";
import {
  type CheckResult,
  type Engine,
  EngineError,
  type IconRow,
  type Measurement,
  type RenderResult,
  type RouteOptions,
} from "./types.js";

export * from "./types.js";

function parse<T>(json: string): T {
  const v = JSON.parse(json) as T & { error?: string; kind?: string };
  if (typeof v.error === "string") {
    throw new EngineError(v.error, v.kind ?? "error");
  }
  return v;
}

class WasmEngine implements Engine {
  render(source: string): RenderResult {
    return parse<RenderResult>(wasm.render(source, true));
  }
  check(source: string): CheckResult {
    return parse(wasm.check(source));
  }
  inspect(source: string, ids: string[] = [], ports = false): string {
    return parse<{ text: string }>(wasm.inspect(source, ids, ports)).text;
  }
  route(source: string, from: string | null, to: string | null, opts: RouteOptions = {}): string {
    return parse<{ text: string }>(
      wasm.route(
        source,
        from ?? undefined,
        to ?? undefined,
        opts.style ?? "corner",
        opts.line,
        opts.radius ?? 8,
        opts.max ?? 3,
      ),
    ).text;
  }
  measure(text: string, size = 14, bold = false): Measurement {
    return parse(wasm.measure(text, size, bold));
  }
  format(source: string): string {
    return parse<{ source: string }>(wasm.format(source)).source;
  }
  ref(topic?: string): string {
    return parse<{ page: string }>(wasm.ref(topic)).page;
  }
  icons(search?: string): IconRow[] {
    return parse<{ icons: IconRow[] }>(wasm.icons(search)).icons;
  }
  version(): string {
    return wasm.version();
  }
}

let loaded: Promise<Engine> | undefined;

/** Loads the wasm once. `bytes` is for Node (tests); in the browser the module URL resolves itself. */
export function loadEngine(bytes?: Uint8Array): Promise<Engine> {
  loaded ??= init(bytes === undefined ? undefined : { module_or_path: bytes })
    .then(() => new WasmEngine())
    .catch((e) => {
      loaded = undefined;
      throw e;
    });
  return loaded;
}
