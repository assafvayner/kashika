<script lang="ts">
  import type { Diagnostic } from "../engine/types.js";

  let { diagnostics }: { diagnostics: Diagnostic[] } = $props();
  let open = $state(false);
  const shown = $derived(diagnostics.filter((d) => !d.suppressed));
  const errors = $derived(shown.filter((d) => d.severity === "error").length);
</script>

{#if shown.length > 0}
  <div class="strip">
    <button class="toggle" onclick={() => (open = !open)}>
      {#if errors > 0}<span class="err">{errors} error{errors === 1 ? "" : "s"}</span>{/if}
      {#if shown.length - errors > 0}<span class="warn"
          >{shown.length - errors} warning{shown.length - errors === 1 ? "" : "s"}</span
        >{/if}
      <span class="chev">{open ? "▾" : "▸"}</span>
    </button>
    {#if open}
      <ul>
        {#each shown as d, i (i)}
          <li class={d.severity}>
            <code>{d.severity}[{d.code}]</code> line {d.span.line}: {d.message}
            {#each d.help ?? [] as h}
              <div class="help">help: {h}</div>
            {/each}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .strip {
    border-top: 1px solid var(--border);
    background: var(--panel);
    font-size: 13px;
    max-height: 40%;
    overflow: auto;
  }
  .toggle {
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 6px 12px;
    cursor: pointer;
    display: flex;
    gap: 12px;
  }
  .err {
    color: var(--error);
  }
  .warn {
    color: var(--warn);
  }
  .chev {
    margin-left: auto;
  }
  ul {
    margin: 0;
    padding: 0 12px 8px 28px;
  }
  li {
    margin: 4px 0;
  }
  .help {
    color: var(--muted);
    margin-left: 12px;
  }
</style>
