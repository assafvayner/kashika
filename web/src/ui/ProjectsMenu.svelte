<script lang="ts">
  import { type Project, loadProject, saveProject } from "../persist/projects.js";
  import type { AppState } from "../state.svelte.js";

  let { app, onclose }: { app: AppState; onclose: () => void } = $props();
  let projects = $state<Project[]>([]);
  let menu = $state<HTMLDivElement | null>(null);

  $effect(() => {
    void app.projects().then((p) => (projects = p));
  });

  async function remove(p: Project) {
    await app.deleteProject(p.id);
    projects = projects.filter((q) => q.id !== p.id);
  }

  function openProject(p: Project) {
    if (p.id !== app.project.id) app.open(p);
    onclose();
  }

  let editing = $state<string | null>(null);

  function startEdit(p: Project, ev: MouseEvent) {
    ev.stopPropagation();
    editing = p.id;
  }

  async function commit(p: Project, value: string) {
    if (editing !== p.id) return;
    editing = null;
    const customTitle = value.trim();
    if (p.id === app.project.id) {
      app.rename(customTitle);
      projects = projects.map((q) => (q.id === p.id ? { ...q, title: app.title } : q));
      return;
    }
    const rec = await loadProject(p.id);
    if (rec === null) return;
    const { customTitle: _drop, ...rest } = rec;
    const next: Project = customTitle === "" ? rest : { ...rest, customTitle };
    next.title = customTitle || p.title;
    await saveProject(next);
    projects = projects.map((q) => (q.id === p.id ? next : q));
  }

  function editKey(p: Project, ev: KeyboardEvent) {
    if (ev.key === "Enter") void commit(p, (ev.currentTarget as HTMLInputElement).value);
    else if (ev.key === "Escape") {
      ev.stopPropagation();
      editing = null;
    }
  }

  function onWindowClick(ev: MouseEvent) {
    const target = ev.target as Node;
    if (menu && !menu.parentElement?.contains(target)) onclose();
  }

  function onWindowKey(ev: KeyboardEvent) {
    if (ev.key === "Escape") onclose();
  }

  function duplicate(p: Project) {
    app.newProject(p.source);
    onclose();
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey} />

<div class="menu" bind:this={menu}>
  <button onclick={() => (app.newProject(), onclose())}>New diagram</button>
  <ul>
    {#each projects as p (p.id)}
      <li class:current={p.id === app.project.id}>
        {#if editing === p.id}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="open"
            autofocus
            value={p.id === app.project.id ? app.title : p.title}
            onkeydown={(ev) => editKey(p, ev)}
            onblur={(ev) => void commit(p, ev.currentTarget.value)}
          />
        {:else}
          <button class="open" onclick={() => openProject(p)}
            >{p.id === app.project.id ? app.title : p.title}</button
          >
        {/if}
        <button title="Rename" onclick={(ev) => startEdit(p, ev)}>✎</button>
        <button title="Duplicate" onclick={() => duplicate(p)}>⧉</button>
        <button title="Delete" onclick={() => void remove(p)}>🗑</button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .menu {
    position: absolute;
    top: 100%;
    left: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    min-width: 260px;
    z-index: 5;
    box-shadow: 0 4px 12px rgb(0 0 0 / 0.15);
  }
  ul {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-height: 50vh;
    overflow: auto;
  }
  li {
    display: flex;
    gap: 4px;
  }
  li.current .open {
    font-weight: 600;
  }
  .open {
    flex: 1;
    text-align: left;
  }
  button {
    border: 0;
    background: none;
    padding: 4px 6px;
    cursor: pointer;
    border-radius: 4px;
  }
  button:hover {
    background: var(--panel);
  }
</style>
