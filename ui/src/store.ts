import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import type {
  Environment,
  InstallState,
  JavaInfo,
  ProgressPayload,
  VersionSummary,
} from "./types";

export type TypeFilter = "release" | "snapshot" | "installed" | "all";

export interface RunningGame {
  pid: number;
  id: string;
}

interface GameExit extends RunningGame {
  code: number | null;
}

export interface State {
  versions: VersionSummary[];
  environment: Environment | null;
  javas: JavaInfo[];
  selected: string | null;
  username: string;
  memory: number;
  isolate: boolean;
  javaPath: string;
  search: string;
  filter: TypeFilter;
  loading: boolean;
  busy: boolean;
  error: string;
  notice: string;
  install: InstallState | null;
  game: RunningGame | null;
}

const KEYS = {
  username: "dlp.username",
  memory: "dlp.memory",
  isolate: "dlp.isolate",
  java: "dlp.java",
};

const stored = {
  username: localStorage.getItem(KEYS.username) ?? "Steve",
  memory: Number(localStorage.getItem(KEYS.memory) ?? 2048),
  isolate: localStorage.getItem(KEYS.isolate) !== "false",
  java: localStorage.getItem(KEYS.java) ?? "",
};

let state: State = {
  versions: [],
  environment: null,
  javas: [],
  selected: null,
  username: stored.username,
  memory: stored.memory,
  isolate: stored.isolate,
  javaPath: stored.java,
  search: "",
  filter: "release",
  loading: false,
  busy: false,
  error: "",
  notice: "",
  install: null,
  game: null,
};

const listeners = new Set<() => void>();

function set(patch: Partial<State>) {
  state = { ...state, ...patch };
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useLauncher(): State {
  return useSyncExternalStore(subscribe, () => state);
}

// ---- 派生数据 ----

export function groupKey(id: string): string {
  const parts = id.split(".");
  if (parts.length >= 3) return `${parts[0]}.${parts[1]}`;
  if (parts.length === 2) return parts[0] === "1" ? id : parts[0];
  return parts[0];
}

export function selectFiltered(s: State): VersionSummary[] {
  const keyword = s.search.trim().toLowerCase();
  return s.versions.filter((v) => {
    if (s.filter === "installed" && !v.installed) return false;
    if (s.filter === "release" && v.kind !== "release") return false;
    if (s.filter === "snapshot" && v.kind === "release") return false;
    return !keyword || v.id.toLowerCase().includes(keyword);
  });
}

export function selectGroups(s: State): { key: string; items: VersionSummary[] }[] {
  const order: string[] = [];
  const map = new Map<string, VersionSummary[]>();
  for (const version of selectFiltered(s)) {
    const key = groupKey(version.id);
    const bucket = map.get(key);
    if (bucket) bucket.push(version);
    else {
      map.set(key, [version]);
      order.push(key);
    }
  }
  return order.map((key) => ({ key, items: map.get(key)! }));
}

export function selectSelectedVersion(s: State): VersionSummary | null {
  return s.versions.find((v) => v.id === s.selected) ?? null;
}

export function selectLatestReleases(s: State): VersionSummary[] {
  return s.versions.filter((v) => v.kind === "release").slice(0, 4);
}

export function javaLabel(javas: JavaInfo[]): string {
  if (javas.length === 0) return "未检测到";
  const max = javas[0].major;
  const min = javas[javas.length - 1].major;
  return min === max ? `Java ${max}` : `Java ${min}–${max}`;
}

// ---- 动作 ----

function persist(patch: Partial<State>) {
  set(patch);
  if (patch.username !== undefined) localStorage.setItem(KEYS.username, patch.username);
  if (patch.memory !== undefined) localStorage.setItem(KEYS.memory, String(patch.memory));
  if (patch.isolate !== undefined) localStorage.setItem(KEYS.isolate, String(patch.isolate));
  if (patch.javaPath !== undefined) localStorage.setItem(KEYS.java, patch.javaPath);
}

async function refresh() {
  set({ loading: true, error: "" });
  try {
    const [versions, environment, javas] = await Promise.all([
      invoke<VersionSummary[]>("list_versions", { includeSnapshots: true }),
      invoke<Environment>("environment"),
      invoke<JavaInfo[]>("detect_java"),
    ]);
    const current = state.selected;
    const fallback =
      versions.find((v) => v.kind === "release" && v.installed)?.id ??
      versions.find((v) => v.kind === "release")?.id ??
      versions[0]?.id ??
      null;
    set({
      versions,
      environment,
      javas,
      selected: current && versions.some((v) => v.id === current) ? current : fallback,
    });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ loading: false });
  }
}

async function installSelected() {
  const target = selectSelectedVersion(state);
  if (!target || state.busy) return;
  set({
    busy: true,
    error: "",
    notice: "",
    install: { id: target.id, stage: "准备中", done: 0, total: 0, percent: 0 },
  });
  try {
    await invoke("install_version", { id: target.id });
    set({
      versions: state.versions.map((v) => (v.id === target.id ? { ...v, installed: true } : v)),
      notice: `${target.id} 安装完成`,
    });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ busy: false, install: null });
  }
}

async function launchSelected() {
  const target = selectSelectedVersion(state);
  if (!target || state.busy) return;
  if (!state.username.trim()) {
    set({ error: "请先填写离线昵称" });
    return;
  }
  set({ busy: true, error: "", notice: "正在拉起游戏…" });
  try {
    const pid = await invoke<number>("launch_game", {
      id: target.id,
      username: state.username.trim(),
      memoryMb: state.memory,
      isolate: state.isolate,
      javaPath: state.javaPath || null,
    });
    set({ game: { pid, id: target.id }, notice: `已启动 ${target.id}，进程 ${pid}` });
  } catch (e) {
    set({ error: String(e), notice: "" });
  } finally {
    set({ busy: false });
  }
}

async function openDir(kind: "data" | "logs" | "versions") {
  try {
    await invoke("open_dir", { kind });
  } catch (e) {
    set({ error: String(e) });
  }
}

let started = false;

async function init() {
  if (started) return;
  started = true;

  await listen<ProgressPayload>("install-progress", (event) => {
    const { stage, done, total } = event.payload;
    if (!state.install) return;
    set({
      install: {
        ...state.install,
        stage,
        done,
        total,
        percent: total > 0 ? done / total : 0,
      },
    });
  });

  // 游戏进程退出：清掉运行状态，否则界面一直显示「运行中 + pid」
  await listen<GameExit>("game-exit", (event) => {
    if (state.game?.pid !== event.payload.pid) return;
    const code = event.payload.code;
    set({
      game: null,
      notice: code === null || code === 0 ? `${event.payload.id} 已退出` : `${event.payload.id} 异常退出（退出码 ${code}）`,
    });
  });

  await refresh();

  try {
    const running = await invoke<RunningGame | null>("running_game");
    if (running) set({ game: running });
  } catch {
    // 老版本后端没有这个命令，忽略
  }
}

export const actions = {
  init,
  refresh,
  installSelected,
  launchSelected,
  openDir,
  setSearch: (search: string) => set({ search }),
  setFilter: (filter: TypeFilter) => set({ filter }),
  setSelected: (selected: string) => set({ selected }),
  setUsername: (username: string) => persist({ username }),
  setMemory: (memory: number) => persist({ memory }),
  setIsolate: (isolate: boolean) => persist({ isolate }),
  setJavaPath: (javaPath: string) => persist({ javaPath }),
};
