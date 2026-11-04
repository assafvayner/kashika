<script lang="ts">
  import type { Document } from "../document/store.svelte.js";
  import { type Engine, EngineError } from "../engine/types.js";
  import Diagnostics from "./Diagnostics.svelte";

  let { doc, engine, onchange }: { doc: Document; engine: Engine; onchange: () => void } = $props();
  let text = $state("");
  let timer: ReturnType<typeof setTimeout> | null = null;
  let fmtError = $state<string | null>(null);

  $effect(() => {
    text = doc.source;
  });

  function apply() {
    doc.set(text, "user");
    onchange();
  }

  function onInput() {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(apply, 300);
  }

  function format() {
    try {
      text = engine.format(text);
      fmtError = null;
      apply();
    } catch (e) {
      fmtError = e instanceof EngineError ? e.message : String(e);
    }
  }
</script>

<div class="drawer">
  <div class="bar">
    <span>Source</span>
    <button onclick={format}>Format</button>
    <button onclick={() => doc.undo()} disabled={!doc.canUndo}>Undo</button>
  </div>
  <textarea bind:value={text} oninput={onInput} spellcheck="false"></textarea>
  {#if fmtError}<pre class="fmt">{fmtError}</pre>{/if}
  <Diagnostics diagnostics={doc.result.diagnostics} />
</div>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
    height: 40%;
    min-height: 160px;
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 12px;
    font-size: 13px;
    background: var(--panel);
  }
  .bar button {
    margin-left: auto;
  }
  .bar button + button {
    margin-left: 0;
  }
  textarea {
    flex: 1;
    resize: none;
    border: 0;
    padding: 8px 12px;
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 13px;
    background: var(--bg);
  }
  .fmt {
    margin: 0;
    padding: 4px 12px;
    font-size: 12px;
    color: var(--error);
  }
</style>
