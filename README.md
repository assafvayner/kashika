# Kashika

A text-to-diagram language for technical designs, written mainly by AI coding agents. There is no auto-layout: you place every box and route every line, and `kashika` checks the geometry for you.

```
diagram "Hello" size (480,240)
box api "API" at (40,40) size (120,60) style=service
box db "Postgres" at api.right+(160,0) size (120,60) pin=left style=datastore
line api.right -> db.left end=arrow label="SQL"
```

```sh
cargo install --path crates/kashika-cli
kashika check hello.kash                 # diagnostics, exit 0/1/2
kashika check hello.kash --format json   # machine-readable
kashika render hello.kash -o hello.svg
kashika measure "Payments API"           # exact label size with the bundled fonts
```

The language is specified in [`docs/spec/language.md`](docs/spec/language.md). Agents should start from [`skills/kashika/SKILL.md`](skills/kashika/SKILL.md).

## License

Apache-2.0, see [LICENSE](LICENSE). The bundled fonts in `crates/kashika-text/fonts/` are under the SIL Open Font License; their license files sit next to them.
