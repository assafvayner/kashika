# Kashika for VS Code

Renders [Kashika](https://github.com/assafvayner/kashika) diagrams in the built-in Markdown preview and highlights `.kash` files.

````markdown
```kashika
diagram "Hello" size (240, 120)

box a "Client" at (20, 30) size (80, 50)
box b "Server" at (140, 30) size (80, 50)

arrow ab a.right -> b.left
```
````

Fences tagged `kashika` or `kash` render as SVG images, through the Kashika renderer compiled to WebAssembly (`crates/kashika-wasm`), so the output is the same as `kashika render` and no binary needs to be installed. Errors and warnings are listed under the diagram, with line numbers in the Markdown file. A diagram with errors still renders, with error markers.

There is no file access from a fence: `import`, `image` and `icons=` directories report not found. Built-in shapes, styles and bundled icons work.

## Development

Needs [wasm-pack](https://github.com/wasm-bindgen/wasm-pack) and Node 20 or later.

```sh
npm install
npm run build          # wasm-pack build of kashika-wasm into wasm/, then tsc
npm test               # markdown-it plugin tests
npm run test:vscode    # runs the extension in VS Code against markdown.api.render
npm run package        # kashika-<version>.vsix
```

Press F5 with this folder open to try it in an Extension Development Host.
