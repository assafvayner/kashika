<script lang="ts">
  import mark from "../../../assets/logo/kashika-mark.svg";
  import { login } from "../auth.js";
  import { exportKash, exportPng, exportSvg } from "../persist/export.js";
  import { encodeShare } from "../persist/share.js";
  import type { AppState } from "../state.svelte.js";
  import ProjectsMenu from "./ProjectsMenu.svelte";

  let {
    app,
    onpick,
    onimport,
  }: { app: AppState; onpick: () => void; onimport: (file: File) => void } = $props();
  let projectsOpen = $state(false);
  let shared = $state<"idle" | "copied" | "failed">("idle");
  let shareUrl = $state<string | null>(null);
  let pngScale = $state(2);
  let fileInput = $state<HTMLInputElement | null>(null);

  async function share() {
    let url = "";
    try {
      url = `${location.origin}${location.pathname}#${await encodeShare(app.doc.source)}`;
      await navigator.clipboard.writeText(url);
      shared = "copied";
      shareUrl = null;
    } catch {
      shared = "failed";
      shareUrl = url;
    }
    setTimeout(() => (shared = "idle"), 1500);
  }

  function title() {
    return app.title.replace(/[^\w.-]+/g, "_") || "diagram";
  }

  function picked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const f = input.files?.[0];
    if (f) onimport(f);
    input.value = "";
  }
</script>

<header>
  <div class="left">
    <a class="brand" href="https://github.com/assafvayner/kashika" target="_blank" rel="noreferrer">
      <img src={mark} alt="" width="22" height="22" />
      <strong>kashika</strong>
    </a>
    <div class="projects">
      <button onclick={() => (projectsOpen = !projectsOpen)}>{app.title} ▾</button>
      {#if projectsOpen}<ProjectsMenu {app} onclose={() => (projectsOpen = false)} />{/if}
    </div>
    <button onclick={() => fileInput?.click()}>Import</button>
    <input bind:this={fileInput} type="file" accept=".kash" hidden onchange={picked} />
    <button onclick={() => exportSvg(title(), app.doc.result.svg ?? "")}>Export SVG</button>
    <button onclick={() => void exportPng(title(), app.doc.result.svg ?? "", pngScale)}
      >Export PNG</button
    >
    <select bind:value={pngScale} title="PNG scale" aria-label="PNG scale">
      <option value={1}>1x</option>
      <option value={2}>2x</option>
      <option value={4}>4x</option>
    </select>
    <button onclick={() => exportKash(title(), app.doc.source)}>Export .kash</button>
    <button onclick={() => void share()}
      >{shared === "copied" ? "Link copied" : shared === "failed" ? "Copy failed" : "Share"}</button
    >
    <button
      class:active={app.drawerOpen}
      onclick={() => (app.drawerOpen = !app.drawerOpen)}
      title="Source (debug only)">Source (debug only)</button
    >
    {#if shareUrl}
      <input
        class="share-url"
        readonly
        value={shareUrl}
        onfocus={(e) => e.currentTarget.select()}
      />
    {/if}
  </div>
  <div class="right">
    <button onclick={onpick}>{app.model ?? "Choose a model"}</button>
    {#if app.session}
      <button onclick={() => app.logout()} title="Log out">{app.session.name || "logged in"}</button
      >
    {:else}
      <button onclick={() => void login()}>Log in</button>
    {/if}
  </div>
</header>

<style>
  .brand {
    display: flex;
    align-items: center;
    gap: 6px;
    color: inherit;
    text-decoration: none;
    margin-right: 4px;
  }
  .brand img {
    display: block;
    border-radius: 5px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
    font-size: 13px;
    gap: 8px;
  }
  .left,
  .right {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .projects {
    position: relative;
  }
  button {
    border: 1px solid var(--border);
    background: var(--bg);
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
  }
  .share-url {
    width: 200px;
    font-size: 12px;
  }
  button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
