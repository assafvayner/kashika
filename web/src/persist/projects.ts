import type { Message } from "../providers/types.js";

export interface ChatEntry {
  role: "user" | "assistant";
  text: string;
  /** Tool step summaries shown under an assistant message. */
  steps?: { summary: string; result: string }[];
}

export interface Project {
  id: string;
  title: string;
  /** Set by rename; overrides the diagram title when present. */
  customTitle?: string;
  source: string;
  chat: ChatEntry[];
  /** The agent's collapsed history, so a reloaded project keeps its context. */
  history?: Message[];
  selection: string | null;
  updated: number;
}

const DB = "kashika";
const STORE = "projects";

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => {
      const store = req.result.createObjectStore(STORE, { keyPath: "id" });
      store.createIndex("updated", "updated");
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

function tx<T>(mode: IDBTransactionMode, f: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return open().then(
    (db) =>
      new Promise<T>((resolve, reject) => {
        try {
          const t = db.transaction(STORE, mode);
          let result: T;
          const req = f(t.objectStore(STORE));
          req.onsuccess = () => {
            result = req.result;
          };
          t.oncomplete = () => {
            db.close();
            resolve(result);
          };
          const fail = () => {
            db.close();
            reject(t.error ?? req.error);
          };
          t.onerror = fail;
          t.onabort = fail;
        } catch (e) {
          db.close();
          reject(e);
        }
      }),
  );
}

export function saveProject(p: Project): Promise<unknown> {
  return tx("readwrite", (s) => s.put(p));
}

export async function loadProject(id: string): Promise<Project | null> {
  return (await tx<Project | undefined>("readonly", (s) => s.get(id))) ?? null;
}

export function deleteProject(id: string): Promise<unknown> {
  return tx("readwrite", (s) => s.delete(id));
}

/** Newest first. */
export async function listProjects(): Promise<Project[]> {
  const all = await tx<Project[]>("readonly", (s) => s.getAll());
  return all.sort((a, b) => b.updated - a.updated);
}

export function newId(): string {
  return crypto.randomUUID();
}
