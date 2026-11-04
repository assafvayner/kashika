<script lang="ts">
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
  let shared = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);

  async function share() {
    const url = `${location.origin}${location.pathname}#${await encodeShare(app.doc.source)}`;
    await navigator.clipboard.writeText(url);
    shared = true;
    setTimeout(() => (shared = false), 1500);
  }

  function title() {
    return app.doc.title.replace(/[^\w.-]+/g, "_") || "diagram";
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
    <strong>kashika</strong>
    <div class="projects">
      <button onclick={() => (projectsOpen = !projectsOpen)}>{app.doc.title} ▾</button>
      {#if projectsOpen}<ProjectsMenu {app} onclose={() => (projectsOpen = false)} />{/if}
    </div>
    <button onclick={() => fileInput?.click()}>Import</button>
    <input bind:this={fileInput} type="file" accept=".kash" hidden onchange={picked} />
    <button onclick={() => exportSvg(title(), app.doc.result.svg ?? "")}>SVG</button>
    <button onclick={() => void exportPng(title(), app.doc.result.svg ?? "")}>PNG</button>
    <button onclick={() => exportKash(title(), app.doc.source)}>.kash</button>
    <button onclick={() => void share()}>{shared ? "Link copied" : "Share"}</button>
    <button
      class:active={app.drawerOpen}
      onclick={() => (app.drawerOpen = !app.drawerOpen)}
      title="Source">&lt;/&gt;</button
    >
  </div>
  <div class="right">
    <button onclick={onpick}>{app.model ?? "Choose a model"}</button>
    {#if app.session}
      <button onclick={() => app.logout()} title="Log out">{app.session.name || "logged in"}</button
      >
    {/if}
  </div>
</header>

<style>
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
  button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
