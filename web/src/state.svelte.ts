import { Agent, type AgentEvent } from "./agent/loop.js";
import { type Session, clearSession } from "./auth.js";
import { Document } from "./document/store.svelte.js";
import type { Engine } from "./engine/types.js";
import {
  type ChatEntry,
  type Project,
  listProjects,
  newId,
  saveProject,
} from "./persist/projects.js";
import { type Settings, loadSettings, saveSettings } from "./persist/settings.js";
import { CustomProvider } from "./providers/custom.js";
import { DEFAULT_MODEL, HuggingFaceProvider } from "./providers/huggingface.js";
import type { ModelRow, Provider } from "./providers/types.js";

const STARTER = `diagram "Untitled"
`;

export class AppState {
  doc = $state<Document>() as Document;
  project = $state<Project>({
    id: newId(),
    title: "Untitled",
    source: STARTER,
    chat: [],
    selection: null,
    updated: Date.now(),
  });
  settings = $state<Settings>(loadSettings());
  session = $state<Session | null>(null);
  models = $state<ModelRow[]>([]);
  modelsError = $state<string | null>(null);
  running = $state(false);
  saveError = $state<string | null>(null);
  drawerOpen = $state(false);
  private agent: Agent | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    readonly engine: Engine,
    session: Session | null,
  ) {
    this.session = session;
    this.doc = new Document(engine, STARTER);
  }

  get provider(): Provider | null {
    if (this.settings.provider === "huggingface") {
      return this.session === null || this.session.expires <= Date.now()
        ? null
        : new HuggingFaceProvider(() => this.session?.token ?? null);
    }
    return this.settings.custom.url === "" ? null : new CustomProvider(this.settings.custom);
  }

  get model(): string | null {
    return this.settings.provider === "huggingface"
      ? (this.settings.hfModel ?? DEFAULT_MODEL)
      : this.settings.custom.model || null;
  }

  updateSettings(patch: Partial<Settings>): void {
    this.settings = { ...this.settings, ...patch };
    saveSettings(this.settings);
    this.dropAgent();
  }

  private dropExpiredSession(): void {
    if (this.session !== null && this.session.expires <= Date.now()) this.logout();
  }

  async loadModels(): Promise<void> {
    this.dropExpiredSession();
    const p = this.provider;
    if (p === null) return;
    this.modelsError = null;
    try {
      this.models = await p.listModels();
      if (
        this.settings.provider === "huggingface" &&
        this.settings.hfModel === null &&
        this.models[0]
      ) {
        this.updateSettings({ hfModel: this.models[0].id });
      }
    } catch (e) {
      this.modelsError = e instanceof Error ? e.message : String(e);
    }
  }

  logout(): void {
    clearSession();
    this.session = null;
    this.dropAgent();
  }

  private dropAgent(): void {
    if (this.agent !== null) {
      this.project = { ...this.project, history: [...this.agent.history] };
      this.agent = null;
    }
  }

  /** Opens a project into the document and chat. */
  open(p: Project): void {
    this.stop();
    this.dropAgent();
    if (this.saveTimer !== null || this.running) {
      if (this.saveTimer !== null) clearTimeout(this.saveTimer);
      this.saveTimer = null;
      this.persist();
    }
    this.running = false;
    this.project = p;
    this.doc = new Document(this.engine, p.source);
    this.doc.selection = p.selection;
  }

  newProject(source = STARTER): Project {
    const p: Project = {
      id: newId(),
      title: "Untitled",
      source,
      chat: [],
      selection: null,
      updated: Date.now(),
    };
    this.open(p);
    this.scheduleSave();
    return p;
  }

  async projects(): Promise<Project[]> {
    return listProjects();
  }

  scheduleSave(): void {
    if (this.saveTimer !== null) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.persist(), 500);
  }

  private persist(): void {
    this.saveNow().then(
      () => {
        this.saveError = null;
      },
      (e: unknown) => {
        this.saveError = e instanceof Error ? e.message : String(e);
      },
    );
  }

  async saveNow(): Promise<void> {
    const record: Project = $state.snapshot({
      ...this.project,
      title: this.doc.title,
      source: this.doc.source,
      selection: this.doc.selection,
      history: this.agent?.history ?? this.project.history,
      updated: Date.now(),
    });
    this.project = record;
    await saveProject($state.snapshot(record));
  }

  /** Runs one chat turn, appending to `project.chat` as events arrive. */
  async send(text: string): Promise<void> {
    this.dropExpiredSession();
    const p = this.provider;
    const model = this.model;
    if (p === null || model === null || this.running) return;
    if (this.agent === null) {
      this.agent = new Agent({
        engine: this.engine,
        doc: this.doc,
        provider: p,
        model,
        budget: this.settings.budget,
      });
      this.agent.history = this.project.history ?? [];
    }
    const id = this.project.id;
    this.running = true;
    const chat = [...this.project.chat, { role: "user", text } as ChatEntry];
    const reply: ChatEntry = { role: "assistant", text: "", steps: [] };
    chat.push(reply);
    this.project = { ...this.project, chat };
    const update = () => {
      if (this.project.id !== id) return;
      this.project = { ...this.project, chat: [...chat] };
    };
    try {
      for await (const ev of this.agent.run(text)) {
        this.apply(reply, ev);
        update();
        if (ev.type === "tool_result" && this.project.id === id) this.scheduleSave();
      }
    } finally {
      if (this.project.id === id) {
        this.running = false;
        this.scheduleSave();
      }
    }
  }

  stop(): void {
    this.agent?.stop();
  }

  private apply(reply: ChatEntry, ev: AgentEvent): void {
    switch (ev.type) {
      case "text":
        reply.text += ev.text;
        break;
      case "tool_start":
        reply.steps!.push({ summary: `${ev.call.name}…`, result: "" });
        break;
      case "tool_result": {
        const step = reply.steps!.at(-1);
        if (step) {
          step.summary = ev.summary;
          step.result = ev.result;
        }
        break;
      }
      case "stopped":
        reply.text += `\n\nStopped after ${ev.steps} steps.`;
        break;
      case "error":
        reply.text += `\n\nError: ${ev.message}`;
        if (
          (/^401/.test(ev.message) || /^not logged in/i.test(ev.message)) &&
          this.settings.provider === "huggingface"
        )
          this.logout();
        break;
      case "done":
        break;
    }
  }
}
