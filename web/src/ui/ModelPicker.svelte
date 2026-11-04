<script lang="ts">
  import { untrack } from "svelte";

  import { login } from "../auth.js";
  import { HuggingFaceProvider } from "../providers/huggingface.js";
  import type { ModelRow } from "../providers/types.js";
  import { REASONING } from "../persist/settings.js";
  import type { AppState } from "../state.svelte.js";

  let { app, onclose }: { app: AppState; onclose: () => void } = $props();
  let tab = $state(untrack(() => app.settings.provider));
  let search = $state("");
  let custom = $state(untrack(() => ({ ...app.settings.custom })));
  let budget = $state(untrack(() => app.settings.budget));
  let reasoning = $state(untrack(() => app.settings.reasoning));
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
    return Math.max(1, Math.min(100, Math.floor(Number(budget) || 30)));
  }

  function close() {
    const b = clampedBudget();
    if (b !== app.settings.budget || reasoning !== app.settings.reasoning)
      app.updateSettings({ budget: b, reasoning });
    onclose();
  }

  function pickHf(id: string) {
    app.updateSettings({
      provider: "huggingface",
      hfModel: id,
      budget: clampedBudget(),
      reasoning,
    });
    onclose();
  }

  function saveCustom() {
    app.updateSettings({
      provider: "openai-compatible",
      custom: { ...custom },
      budget: clampedBudget(),
      reasoning,
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
        <button class="primary" onclick={() => void login()}>Log in</button>
        <p class="muted">
          Opens in a new tab when running inside huggingface.co; come back here or continue in the
          new tab.
        </p>
      {:else}
        <input class="search" placeholder="Search models" bind:value={search} />
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
      <button
        class="primary"
        onclick={saveCustom}
        disabled={custom.url === "" || custom.model === ""}>Use this endpoint</button
      >
    {/if}
    <label class="budget"
      >Tool calls per turn <input type="number" min="1" max="100" bind:value={budget} /></label
    >
    <label class="budget reasoning"
      >Reasoning effort
      <select bind:value={reasoning}>
        {#each REASONING as r (r)}<option value={r}>{r === "default" ? "model default" : r}</option
          >{/each}
      </select></label
    >
    <p class="muted">
      Low keeps thinking models from planning the whole layout in their head. Set it to model
      default for endpoints that reject <code>reasoning_effort</code>.
    </p>
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
    border-radius: 14px;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.18);
    width: min(560px, 92vw);
    max-height: 80vh;
    overflow: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  button {
    font: inherit;
    cursor: pointer;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--fg);
    padding: 6px 14px;
    transition:
      background 0.12s,
      border-color 0.12s,
      color 0.12s;
  }
  button:hover {
    background: var(--panel);
  }
  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px;
    border-radius: 999px;
    background: var(--panel);
  }
  .tabs button {
    border: 0;
    background: none;
    color: var(--muted);
    padding: 6px 14px;
  }
  .tabs button.active {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.12);
  }
  .tabs .close {
    margin-left: auto;
    width: 30px;
    height: 30px;
    padding: 0;
    font-size: 18px;
    line-height: 1;
  }
  .tabs .close:hover {
    background: var(--bg);
  }
  .primary {
    align-self: flex-start;
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .primary:hover {
    background: color-mix(in srgb, var(--accent) 88%, black);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 40vh;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    text-align: left;
    border: 1px solid transparent;
    border-radius: 8px;
    background: none;
    padding: 7px 10px;
  }
  li button:hover {
    background: var(--panel);
  }
  li button.active {
    background: color-mix(in srgb, var(--accent) 10%, var(--bg));
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--accent);
  }
  li button.active::after {
    content: "✓";
    margin-left: auto;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--muted);
  }
  input {
    font: inherit;
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 10px;
    background: var(--bg);
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
    border-color: transparent;
  }
  .budget {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .budget input {
    width: 80px;
  }
  .reasoning {
    padding-top: 8px;
    border-top: none;
  }
  .muted {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
  p {
    margin: 0;
  }
  .err {
    color: var(--error);
  }
</style>
