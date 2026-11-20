<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="../assets/logo/kashika-logo-dark.svg">
    <img src="../assets/logo/kashika-logo.svg" alt="Kashika" height="60">
  </picture>
  web app
</h1>

An agent-first diagram tool, deployed as a Hugging Face static Space. The user describes a diagram in chat; an in-browser agent writes Kashika source through `kashika-wasm` and the canvas shows the result. There is no server: models are called from the browser through Hugging Face inference providers (after OAuth login) or any OpenAI-compatible endpoint.

## Setup

Rust stable with the `wasm32-unknown-unknown` target, [wasm-pack](https://github.com/wasm-bindgen/wasm-pack), Node 22+.

```sh
npm install
npm run build:wasm   # wasm-pack build of crates/kashika-wasm into src/engine/pkg (gitignored)
npm run dev
```

## Commands

- `npm run check`: svelte-check.
- `npm test`: vitest (loads the wasm from `src/engine/pkg`, so run `build:wasm` first).
- `npm run build`: wasm then Vite, into `dist/`.
- `npm run fmt` / `fmt:check`: prettier.

## Login in local development

In the Space, `hf_oauth: true` in `space/README.md` makes the Hub inject the OAuth client id. Locally, create an OAuth app at https://huggingface.co/settings/applications with the redirect URL `http://localhost:5173/` and the `openid`, `profile` and `inference-api` scopes, then put its id in `.env.local`:

```
VITE_OAUTH_CLIENT_ID=...
VITE_OAUTH_REDIRECT_URL=http://localhost:5173/
```

The `VITE_OAUTH_*` variables are read only by `npm run dev`; built output gets the client id from the Space.

Without a login, a custom OpenAI-compatible endpoint (base URL, token, model) works from the model picker. That is also how to use Claude, OpenAI or Gemini for now, through an OpenAI-compatible gateway; native providers are a planned follow-up.

## Deploy

Create a static Space, then:

```sh
pip install -U huggingface_hub
KASHIKA_SPACE=owner/kashika HF_TOKEN=... ./deploy.sh
```

The script builds, copies `space/README.md` (the Space metadata) into `dist/`, and uploads `dist/` with `hf upload`.
