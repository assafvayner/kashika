<script lang="ts">
  import type { ChatEntry } from "../persist/projects.js";

  let {
    chat,
    running,
    ready,
    selection,
    onsend,
    onstop,
    onclear,
    error = null,
    onretry = () => {},
    onlogin = () => {},
    onpick = () => {},
  }: {
    chat: ChatEntry[];
    running: boolean;
    ready: boolean;
    selection: string | null;
    onsend: (text: string) => void;
    onstop: () => void;
    onclear: () => void;
    error?: { message: string; status: number | null } | null;
    onretry?: () => void;
    onlogin?: () => void;
    onpick?: () => void;
  } = $props();

  let draft = $state("");
  let list = $state<HTMLDivElement | null>(null);
  let input = $state<HTMLTextAreaElement | null>(null);
  let openStep = $state<string | null>(null);
  let elapsed = $state(0);

  $effect(() => {
    if (!running) return;
    const start = Date.now();
    elapsed = 0;
    const t = setInterval(() => (elapsed = Math.floor((Date.now() - start) / 1000)), 1000);
    return () => clearInterval(t);
  });

  const status = $derived.by(() => {
    const m = chat.at(-1);
    if (!running || m?.role !== "assistant") return "";
    const step = m.steps?.at(-1);
    if (step && step.summary.endsWith("…")) return `running ${step.summary.slice(0, -1)}`;
    if (m.text) return "writing";
    if (m.thinking) return "thinking";
    return "waiting for the model";
  });

  $effect(() => {
    void chat.length;
    void chat.at(-1)?.text;
    void chat.at(-1)?.thinking;
    list?.scrollTo?.({ top: list.scrollHeight });
  });

  function scrollThinking(node: HTMLElement) {
    $effect(() => {
      void chat.at(-1)?.thinking;
      node.scrollTop = node.scrollHeight;
    });
  }

  $effect(() => {
    void draft;
    if (!input) return;
    input.style.height = "auto";
    input.style.height = `${input.scrollHeight}px`;
  });

  function send() {
    const text = draft.trim();
    if (text === "" || running || !ready) return;
    draft = "";
    onsend(text);
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Enter" && !ev.shiftKey) {
      ev.preventDefault();
      send();
    }
  }
</script>

<div class="chat">
  <div class="list" bind:this={list}>
    {#each chat as m, i (i)}
      <div class="msg {m.role}">
        {#if m.thinking}
          <details class="thinking" open={running && i === chat.length - 1}>
            <summary>Thinking</summary>
            <div class="reasoning" use:scrollThinking>{m.thinking}</div>
          </details>
        {/if}
        {#if m.steps && m.steps.length > 0}
          <div class="steps">
            {#each m.steps as s, j (j)}
              <button
                class="step"
                onclick={() => (openStep = openStep === `${i}:${j}` ? null : `${i}:${j}`)}
                >{s.summary}</button
              >
              {#if openStep === `${i}:${j}`}<pre>{s.result}</pre>{/if}
            {/each}
          </div>
        {/if}
        {#if m.text.trim()}<div class="text">{m.text.trim()}</div>{/if}
      </div>
    {/each}
    {#if running}<div class="working">{status}… {elapsed}s</div>{/if}
    {#if error && !running}
      {#if error.status === 429}
        <p class="warn">Rate limited by the model provider. Wait a moment, then retry.</p>
      {/if}
      <button class="retry" onclick={onretry}>Retry</button>
    {/if}
  </div>
  <div class="composer">
    {#if !ready}
      <p class="hint">Not connected. Log in or add an endpoint to start.</p>
      <div class="connect">
        <button onclick={onlogin}>Log in with Hugging Face</button>
        <button onclick={onpick}>Use a custom endpoint</button>
      </div>
    {/if}
    {#if selection}
      <span class="chip"
        ><span>{selection}</span>
        <button onclick={onclear} title="Clear selection">×</button></span
      >
    {/if}
    <textarea
      bind:this={input}
      bind:value={draft}
      onkeydown={onKey}
      rows="2"
      placeholder={ready ? "Describe the diagram or a change…" : "Not connected"}
      disabled={!ready}
    ></textarea>
    {#if running}
      <button class="send" onclick={onstop}>Stop</button>
    {:else}
      <button class="send" onclick={send} disabled={!ready}>Send</button>
    {/if}
  </div>
</div>

<style>
  .chat {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-left: 1px solid var(--border);
    background: var(--panel);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .msg {
    max-width: 92%;
    border-radius: 10px;
    padding: 8px 10px;
    font-size: 14px;
    white-space: pre-wrap;
  }
  .msg.user {
    align-self: flex-end;
    background: var(--accent);
    color: #fff;
  }
  .msg.assistant {
    align-self: flex-start;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 6px;
  }
  .step {
    text-align: left;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }
  pre {
    font-size: 11px;
    max-height: 200px;
    overflow: auto;
    background: var(--panel);
    padding: 6px;
    margin: 2px 0 6px;
  }
  .working {
    color: var(--muted);
    font-size: 12px;
  }
  .thinking {
    margin-bottom: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .thinking summary {
    cursor: pointer;
    user-select: none;
  }
  .reasoning {
    max-height: 160px;
    overflow: auto;
    margin-top: 4px;
    padding-left: 8px;
    border-left: 2px solid var(--border);
    white-space: pre-wrap;
    font-size: 12px;
    line-height: 1.4;
  }
  .composer {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px;
    border-top: 1px solid var(--border);
  }
  .connect {
    display: flex;
    gap: 6px;
    width: 100%;
  }
  .connect button {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px;
    background: var(--bg);
    cursor: pointer;
  }
  .hint {
    width: 100%;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .chip {
    border: 1px solid #e67e22;
    color: #e67e22;
    border-radius: 10px;
    padding: 0 8px;
    font-size: 12px;
    width: fit-content;
    max-width: 100%;
    flex-basis: 100%;
  }
  .warn {
    margin: 0;
    font-size: 13px;
    color: var(--warn);
  }
  .retry {
    align-self: flex-start;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px 10px;
    background: var(--bg);
    cursor: pointer;
  }
  .chip button {
    border: 0;
    background: none;
    cursor: pointer;
    color: inherit;
  }
  textarea {
    flex: 1;
    resize: none;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    background: var(--bg);
    min-width: 0;
    width: 100%;
    max-height: 50vh;
    overflow-y: auto;
    box-sizing: border-box;
    font: inherit;
    line-height: 1.4;
  }
  .send {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px;
    background: var(--bg);
    cursor: pointer;
  }
</style>
