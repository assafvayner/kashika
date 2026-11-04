# kashika (TypeScript)

An experimental TypeScript port of the Rust crates. It mirrors them file by file under `src/<crate>/` (`src/diag` is `kashika-diag`, `src/geom` is `kashika-geom`, and so on).

## Setup

Node 22+, then `npm install` in `ts/`.

## Commands

- `npm run check`: type-check.
- `npm test`: all tests, including goldens.
- `npm run golden`: golden cases only, against the shared `tests/golden` corpus.
- `npm run parity`: parity script.
- `npm run gen-icons`: regenerate bundled icon data (needs cargo).
- `npm run bench`: compare with `kashika-wasm` (needs `wasm-pack build crates/kashika-wasm --target nodejs`).
- `node bin/kashika.js <subcommand>`: the CLI.

## Known divergences from the Rust build

- User-supplied SVG files are sanitized by `src/model/assets.ts` instead of usvg, so render goldens that embed them differ (`test/golden/divergent.ts` lists them; `examples/network` too).
- PNG and JPEG output goes through `@resvg/resvg-js` and `jpeg-js` and is compared by dimensions, not bytes. WebP output is unsupported.
- `src/wasm-api.ts` resolves file references from the cwd, whereas the real wasm build has no file system.
- Float trig and `hypot` match macOS libm and may differ in the last bit elsewhere (see `src/diag/rust.ts`).

Benchmark (`bench/results.md`): of 173 diagrams compared, all 173 SVGs are byte-identical to `kashika-wasm`; the sum of medians is 161.93 ms against 97.03 ms (ratio 1.67), and cold start is 99.28 ms against 30.71 ms.
