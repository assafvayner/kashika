<script lang="ts">
  import type { Project } from "../persist/projects.js";
  import { deleteProject } from "../persist/projects.js";
  import type { AppState } from "../state.svelte.js";

  let { app, onclose }: { app: AppState; onclose: () => void } = $props();
  let projects = $state<Project[]>([]);

  $effect(() => {
    void app.projects().then((p) => (projects = p));
  });

  async function remove(p: Project) {
    await deleteProject(p.id);
    projects = projects.filter((q) => q.id !== p.id);
    if (p.id === app.project.id) app.newProject();
  }

  function duplicate(p: Project) {
    app.newProject(p.source);
    onclose();
  }
</script>

<div class="menu">
  <button onclick={() => (app.newProject(), onclose())}>New diagram</button>
  <ul>
    {#each projects as p (p.id)}
      <li class:current={p.id === app.project.id}>
        <button class="open" onclick={() => (app.open(p), onclose())}>{p.title}</button>
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
