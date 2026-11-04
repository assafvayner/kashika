<script lang="ts">
  import { login } from "../auth.js";
  import type { AppState } from "../state.svelte.js";

  let { app, onclose }: { app: AppState; onclose: () => void } = $props();
  let tab = $state(app.settings.provider);
  let search = $state("");
  let custom = $state({ ...app.settings.custom });
  let budget = $state(app.settings.budget);

  const shown = $derived(
    app.models.filter((m) => m.id.toLowerCase().includes(search.toLowerCase())),
  );

  function pickHf(id: string) {
    app.updateSettings({ provider: "huggingface", hfModel: id, budget });
    onclose();
  }

  function saveCustom() {
    app.updateSettings({ provider: "openai-compatible", custom: { ...custom }, budget });
    void app.loadModels();
    onclose();
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <div
    class="dialog"
    role="dialog"
    tabindex="-1"
    aria-label="Model"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="tabs">
      <button class:active={tab === "huggingface"} onclick={() => (tab = "huggingface")}
        >Hugging Face</button
      >
      <button class:active={tab === "openai-compatible"} onclick={() => (tab = "openai-compatible")}
        >Custom endpoint</button
      >
      <button class="close" onclick={onclose}>×</button>
    </div>
    {#if tab === "huggingface"}
      {#if app.session === null}
        <p>Log in with Hugging Face to use inference providers.</p>
        <button onclick={() => void login()}>Log in</button>
      {:else}
        <input placeholder="Search models" bind:value={search} />
        {#if app.modelsError}<p class="err">{app.modelsError}</p>{/if}
        <ul>
          {#each shown as m (m.id)}
            <li>
              <button class:active={m.id === app.settings.hfModel} onclick={() => pickHf(m.id)}>
                {m.id} <span class="muted">{m.provider}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <label
        >Base URL <input bind:value={custom.url} placeholder="https://api.example.com/v1" /></label
      >
      <label>Token <input bind:value={custom.token} type="password" /></label>
      <label>Model <input bind:value={custom.model} placeholder="model id" /></label>
      <p class="muted">Saved in this browser's localStorage, token included.</p>
      <button onclick={saveCustom} disabled={custom.url === "" || custom.model === ""}
        >Use this endpoint</button
      >
    {/if}
    <label class="budget"
      >Tool calls per turn <input type="number" min="1" max="100" bind:value={budget} /></label
    >
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.35);
    display: grid;
    place-items: center;
    z-index: 10;
  }
  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    width: min(560px, 92vw);
    max-height: 80vh;
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .tabs {
    display: flex;
    gap: 8px;
  }
  .tabs .close {
    margin-left: auto;
  }
  button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 40vh;
    overflow: auto;
  }
  li button {
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 6px 8px;
    cursor: pointer;
  }
  li button:hover {
    background: var(--panel);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  input {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
    background: var(--bg);
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    color: var(--error);
  }
</style>
