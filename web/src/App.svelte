<script lang="ts">
  import { onMount } from "svelte";

  import { completeLogin } from "./auth.js";
  import { loadEngine } from "./engine/index.js";
  import { listProjects } from "./persist/projects.js";
  import { decodeShare } from "./persist/share.js";
  import { AppState } from "./state.svelte.js";
  import Canvas from "./ui/Canvas.svelte";
  import Chat from "./ui/Chat.svelte";
  import Diagnostics from "./ui/Diagnostics.svelte";
  import ModelPicker from "./ui/ModelPicker.svelte";
  import SourceDrawer from "./ui/SourceDrawer.svelte";
  import TopBar from "./ui/TopBar.svelte";

  let app = $state<AppState | null>(null);
  let pickerOpen = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  function notify(msg: string) {
    notice = msg;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = null), 3000);
  }

  onMount(async () => {
    try {
      const [engine, session] = await Promise.all([loadEngine(), completeLogin()]);
      const a = new AppState(engine, session);
      const shared = await decodeShare(location.hash.slice(1));
      if (shared !== null) {
        a.newProject(shared);
        history.replaceState(null, "", location.pathname);
      } else {
        const [latest] = await listProjects();
        if (latest) a.open(latest);
        else a.newProject();
      }
      app = a;
      void a.loadModels();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  $effect(() => {
    if (app?.loginRequested) {
      pickerOpen = true;
      app.loginRequested = false;
    }
  });

  $effect(() => {
    if (!app) return;
    void app.doc.version;
    void app.doc.selection;
    app.scheduleSave();
  });

  async function importFile(file: File) {
    app?.newProject(await file.text());
  }

  function onDrop(ev: DragEvent) {
    ev.preventDefault();
    const f = ev.dataTransfer?.files[0];
    if (!f) return;
    if (f.name.endsWith(".kash")) void importFile(f);
    else notify("Only .kash files can be imported.");
  }
</script>

{#if error}
  <p class="error">Failed to start: {error}</p>
{:else if app}
  <div class="app" role="presentation" ondragover={(e) => e.preventDefault()} ondrop={onDrop}>
    <TopBar {app} onpick={() => (pickerOpen = true)} onimport={importFile} />
    <div class="body">
      <div class="left">
        <div class="canvas">
          <Canvas doc={app.doc} />
          {#if notice}<div class="notice">{notice}</div>{/if}
        </div>
        {#if app.drawerOpen}
          <SourceDrawer doc={app.doc} engine={app.engine} onchange={() => app?.scheduleSave()} />
        {:else}
          <Diagnostics diagnostics={app.doc.result.diagnostics} />
        {/if}
      </div>
      <div class="right">
        <Chat
          chat={app.project.chat}
          running={app.running}
          ready={app.provider !== null && app.model !== null}
          selection={app.doc.selection}
          onsend={(t) => void app?.send(t)}
          onstop={() => app?.stop()}
          onretry={() => void app?.retry()}
          onclear={() => app && (app.doc.selection = null)}
        />
      </div>
    </div>
  </div>
  {#if pickerOpen}<ModelPicker {app} onclose={() => (pickerOpen = false)} />{/if}
{:else}
  <p class="loading">Loading…</p>
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .left {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .notice {
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 12px;
    font-size: 13px;
  }
  .right {
    width: min(420px, 40%);
  }
  .loading,
  .error {
    padding: 24px;
    color: var(--muted);
  }
  .error {
    color: var(--error);
  }
</style>
