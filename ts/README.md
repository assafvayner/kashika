# kashika (TypeScript)

A TypeScript port of the Kashika Rust workspace. The Rust crates under `crates/` are the
specification: each module in `src/` ports one crate (`src/diag` is `kashika-diag`,
`src/geom` is `kashika-geom`, and so on) function by function, with the same results bit
for bit.

```sh
npm install
npm run check   # tsc
npm test        # vitest
```
