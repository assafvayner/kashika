<script lang="ts">
  import { untrack } from "svelte";

  import { login } from "../auth.js";
  import { HuggingFaceProvider } from "../providers/huggingface.js";
  import type { ModelRow } from "../providers/types.js";
  import type { AppState } from "../state.svelte.js";

  let { app, onclose }: { app: AppState; onclose: () => void } = $props();
  let tab = $state(untrack(() => app.settings.provider));
  let search = $state("");
  let custom = $state(untrack(() => ({ ...app.settings.custom })));
  let budget = $state(untrack(() => app.settings.budget));
  let hfModels = $state<ModelRow[]>([]);
  let hfError = $state<string | null>(null);
  let dialog = $state<HTMLDivElement | null>(null);

  const hfActive = $derived(app.settings.provider === "huggingface");
  const hfList = $derived(hfActive ? app.models : hfModels);
  const hfErr = $derived(hfActive ? app.modelsError : hfError);
  const shown = $derived(hfList.filter((m) => m.id.toLowerCase().includes(search.toLowerCase())));

  $effect(() => {
    dialog?.focus();
  });

  $effect(() => {
    if (tab !== "huggingface" || hfActive || app.session === null) return;
    let live = true;
    hfError = null;
    new HuggingFaceProvider(() => app.session?.token ?? null)
      .listModels()
      .then((m) => live && (hfModels = m))
      .catch((e) => live && (hfError = e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  });

  function clampedBudget(): number {
    return Math.max(1, Math.min(100, Math.floor(Number(budget) || 20)));
  }

  function close() {
    const b = clampedBudget();
    if (b !== app.settings.budget) app.updateSettings({ budget: b });
    onclose();
  }

  function pickHf(id: string) {
    app.updateSettings({ provider: "huggingface", hfModel: id, budget: clampedBudget() });
    onclose();
  }

  function saveCustom() {
    app.updateSettings({
      provider: "openai-compatible",
      custom: { ...custom },
      budget: clampedBudget(),
    });
    void app.loadModels();
    onclose();
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape") close();
  }
</script>

<div class="backdrop" role="presentation" onclick={close}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <div
    class="dialog"
    role="dialog"
    tabindex="-1"
    aria-label="Model"
    bind:this={dialog}
    onkeydown={onKey}
    onclick={(e) => e.stopPropagation()}
  >
    <div class="tabs">
      <button class:active={tab === "huggingface"} onclick={() => (tab = "huggingface")}
        >Hugging Face</button
      >
      <button class:active={tab === "openai-compatible"} onclick={() => (tab = "openai-compatible")}
        >Custom endpoint</button
      >
      <button class="close" onclick={close}>×</button>
    </div>
    {#if tab === "huggingface"}
      {#if app.session === null}
        <p>Log in with Hugging Face to use inference providers.</p>
        <button onclick={() => void login()}>Log in</button>
      {:else}
        <input placeholder="Search models" bind:value={search} />
        {#if hfErr}<p class="err">{hfErr}</p>{/if}
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
