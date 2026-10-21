import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import type {
  Account,
  AccountsPayload,
  AppSettings,
  Candidate,
  DownloadView,
  Environment,
  InstallState,
  JavaInfo,
  LoaderKind,
  ProgressPayload,
  MarketHit,
  MarketKind,
  MarketPage,
  ModEntry,
  SaveEntry,
  Theme,
  VersionSettings,
  VersionSummary,
} from "./types";

export type TypeFilter = "release" | "snapshot" | "installed" | "local" | "all";

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
  /** 离线账号列表与当前账号 id */
  accounts: Account[];
  currentAccount: string | null;
  /** 主页选中的「每次启动用哪个版本」 */
  defaultVersion: string | null;
  /** 下载页视图：列表 / 版本详情 / 该版本的设置 */
  downloadView: DownloadView;
  downloadTab: "versions" | "market";
  search: string;
  filter: TypeFilter;
  loading: boolean;
  busy: boolean;
  error: string;
  notice: string;
  install: InstallState | null;
  game: RunningGame | null;

  /** 当前选中版本的设置（已合并全局默认） */
  settings: VersionSettings | null;
  /** 正在编辑的 key："*" = 全局默认 */
  settingsKey: string;

  // 安装选项
  loader: LoaderKind;
  loaderVersion: string;
  optifine: boolean;
  optifineEdition: string;
  loaderOptions: Candidate[];
  optifineOptions: Candidate[];

  // 资源搜索（Modrinth）
  marketQuery: string;
  marketKind: MarketKind;
  marketHits: MarketHit[];
  marketTotal: number;
  marketLoading: boolean;
  baseVersion: string;

  // 内容管理
  mods: ModEntry[];
  saves: SaveEntry[];
  logs: string[];
  logName: string;
  logText: string;
  theme: Theme;
}

/** 当前账号昵称（一个账号都没有时退回 Steve） */
export function currentName(s: State): string {
  const id = s.currentAccount;
  const found = id ? s.accounts.find((a) => a.id === id) : undefined;
  return found?.name ?? "Steve";
}

const EMPTY_SETTINGS: VersionSettings = {
  memory_mb: 2048,
  java_path: null,
  isolate: true,
  extra_jvm_args: [],
};

let state: State = {
  versions: [],
  environment: null,
  javas: [],
  selected: null,
  accounts: [],
  currentAccount: null,
  defaultVersion: null,
  downloadView: { name: "list" },
  downloadTab: "versions",
  search: "",
  filter: "release",
  loading: false,
  busy: false,
  error: "",
  notice: "",
  install: null,
  game: null,
  settings: null,
  settingsKey: "",
  loader: "none",
  loaderVersion: "",
  optifine: false,
  optifineEdition: "",
  loaderOptions: [],
  optifineOptions: [],
  marketQuery: "",
  marketKind: "mod",
  marketHits: [],
  marketTotal: 0,
  marketLoading: false,
  baseVersion: "",
  mods: [],
  saves: [],
  logs: [],
  logName: "",
  logText: "",
  theme: (localStorage.getItem("dlp.theme") as Theme) ?? "light",
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
  return s.versions.filter((v) => v.local || v.kind !== undefined)
    .filter((v) => {
      if (s.filter === "installed" && !v.installed) return false;
      if (s.filter === "local" && !v.local) return false;
      if (s.filter === "release" && v.kind !== "release") return false;
      if (s.filter === "snapshot" && v.kind === "release") return false;
      return !keyword || v.id.toLowerCase().includes(keyword);
    });
}

export function selectGroups(s: State): { key: string; items: VersionSummary[] }[] {
  const order: string[] = [];
  const map = new Map<string, VersionSummary[]>();
  for (const version of selectFiltered(s)) {
    const key = version.april_fools ? "愚人节" : groupKey(version.id);
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
  return s.versions.filter((v) => v.kind === "release" && !v.local).slice(0, 4);
}

export function javaLabel(javas: JavaInfo[]): string {
  if (javas.length === 0) return "未检测到";
  const max = javas[0].major;
  const min = javas[javas.length - 1].major;
  return min === max ? `Java ${max}` : `Java ${min}–${max}`;
}

// ---- 动作 ----

async function refresh() {
  set({ loading: true, error: "" });
  try {
    const [versions, environment, javas, accounts, cfg] = await Promise.all([
      invoke<VersionSummary[]>("list_versions", { includeSnapshots: true }),
      invoke<Environment>("environment"),
      invoke<JavaInfo[]>("detect_java"),
      invoke<AccountsPayload>("accounts"),
      invoke<AppSettings>("app_settings"),
    ]);
    const current = state.selected;
    const fallback =
      versions.find((v) => v.installed && v.kind === "release")?.id ??
      versions.find((v) => v.kind === "release")?.id ??
      versions[0]?.id ??
      null;
    set({
      versions,
      environment,
      javas,
      accounts: accounts.list,
      currentAccount: accounts.current,
      defaultVersion: cfg.default_version,
      selected: current && versions.some((v) => v.id === current) ? current : fallback,
    });
    if (state.selected) void loadSettings(state.selected);
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ loading: false });
  }
}

/** 选中版本变化时把它的设置拉下来 */
async function loadSettings(id: string) {
  set({ settingsKey: id });
  try {
    const settings = await invoke<VersionSettings>("version_settings", { id });
    set({ settings });
  } catch (e) {
    set({ error: String(e), settings: { ...EMPTY_SETTINGS } });
  }
}

async function saveSettings(id: string, settings: VersionSettings) {
  set({ settings });
  try {
    await invoke("save_version_settings", { id, settings });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadLoaderOptions(mc: string, kind: LoaderKind) {
  if (kind === "none") {
    set({ loaderOptions: [], loaderVersion: "" });
    return;
  }
  set({ loader: kind, loaderVersion: "", loaderOptions: [] });
  try {
    const list = await invoke<Candidate[]>("loader_candidates", { mc, kind });
    set({ loaderOptions: list, loaderVersion: list[0]?.version ?? "" });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadOptifineOptions(mc: string, enabled: boolean) {
  set({ optifine: enabled });
  if (!enabled) {
    set({ optifineOptions: [], optifineEdition: "" });
    return;
  }
  try {
    const list = await invoke<Candidate[]>("optifine_candidates", { mc });
    set({ optifineOptions: list, optifineEdition: list[0]?.version ?? "" });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function installSelected() {
  const target = selectSelectedVersion(state);
  if (!target || state.busy) {
    return;
  }
  set({
    busy: true,
    error: "",
    notice: "",
    install: { id: target.id, stage: "准备中", done: 0, total: 0, percent: 0 },
  });

  let finalId = target.id;
  try {
    await invoke("install_version", { id: target.id });
    if (state.loader !== "none" && state.loaderVersion) {
      set({ install: { ...state.install!, stage: `安装 ${state.loader}`, percent: 0 } });
      finalId = await invoke<string>("install_loader", {
        mc: target.id,
        kind: state.loader,
        version: state.loaderVersion,
      });
    }
    if (state.optifine && state.optifineEdition) {
      set({ install: { ...state.install!, stage: "安装 OptiFine", percent: 0 } });
      await invoke<string>("install_optifine", {
        mc: target.id,
        edition: state.optifineEdition,
      });
    }
    set({ notice: finalId === target.id ? `${target.id} 安装完成` : `已安装 ${finalId}` });
    await refresh();
    set({ selected: finalId });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ busy: false, install: null });
  }
}

async function launchSelected() {
  const target = selectSelectedVersion(state);
  if (!target || state.busy) return;
  const name = currentName(state);
  set({ busy: true, error: "", notice: "正在拉起游戏…" });
  try {
    const pid = await invoke<number>("launch_game", {
      id: target.id,
      username: name,
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

  await listen<GameExit>("game-exit", (event) => {
    if (state.game?.pid !== event.payload.pid) return;
    const code = event.payload.code;
    set({
      game: null,
      notice:
        code === null || code === 0
          ? `${event.payload.id} 已退出`
          : `${event.payload.id} 异常退出（退出码 ${code}）`,
    });
  });

  await refresh();
  if (state.selected) {
    void loadContent(state.selected);
    void loadBaseVersion(state.selected);
  }

  try {
    const running = await invoke<RunningGame | null>("running_game");
    if (running) set({ game: running });
  } catch {
    // 老后端没有这个命令
  }
}

async function loadContent(id: string) {
  try {
    const [mods, saves] = await Promise.all([
      invoke<ModEntry[]>("list_mods", { id }),
      invoke<SaveEntry[]>("list_saves", { id }),
    ]);
    set({ mods, saves });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function toggleMod(id: string, name: string, enabled: boolean) {
  try {
    await invoke("set_mod_enabled", { id, name, enabled });
    await loadContent(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function deleteMod(id: string, name: string) {
  try {
    await invoke("delete_mod", { id, name });
    await loadContent(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadLogs(preferred?: string) {
  try {
    const logs = await invoke<string[]>("list_logs");
    const name = preferred || state.logName || logs[0] || "";
    set({ logs, logName: name });
    if (name) {
      const logText = await invoke<string>("read_log", { name, lines: 400 });
      set({ logText });
    }
  } catch (e) {
    set({ error: String(e) });
  }
}

async function openVersionDir(id: string, kind: "root" | "mods" | "saves" | "instance") {
  try {
    await invoke("open_version_dir", { id, kind });
  } catch (e) {
    set({ error: String(e) });
  }
}

function setTheme(theme: Theme) {
  localStorage.setItem("dlp.theme", theme);
  set({ theme });
}

async function loadBaseVersion(id: string) {
  try {
    const baseVersion = await invoke<string>("base_version", { id });
    set({ baseVersion });
  } catch {
    set({ baseVersion: id });
  }
}

function loaderOf(id: string): string {
  const lower = id.toLowerCase();
  if (lower.includes("neoforge")) return "neoforge";
  if (lower.includes("fabric")) return "fabric";
  if (lower.includes("quilt")) return "quilt";
  if (lower.includes("forge")) return "forge";
  return "";
}

async function searchMarket() {
  const gameId = state.selected;
  if (!gameId) return;
  const mc = state.baseVersion || gameId;
  set({ marketLoading: true, error: "" });
  try {
    const page = await invoke<MarketPage>("search_market", {
      query: state.marketQuery,
      kind: state.marketKind,
      mc,
      loader: loaderOf(gameId) || null,
      offset: 0,
      limit: 30,
    });
    set({ marketHits: page.hits, marketTotal: page.total });
  } catch (e) {
    set({ error: String(e), marketHits: [] });
  } finally {
    set({ marketLoading: false });
  }
}

async function installMarket(project: string, kind: MarketKind) {
  const gameId = state.selected;
  if (!gameId) return;
  set({ busy: true, error: "", notice: "" });
  try {
    const file = await invoke<string>("install_market", { id: gameId, project, kind });
    set({ notice: `已安装 ${file}` });
    await loadContent(gameId);
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ busy: false });
  }
}

async function installJava(major: number) {
  set({ busy: true, error: "", notice: `正在下载 Java ${major}…` });
  try {
    const path = await invoke<string>("install_java", { major });
    const javas = await invoke<JavaInfo[]>("detect_java");
    set({ javas, notice: `Java ${major} 已就绪: ${path}` });
  } catch (e) {
    set({ error: String(e), notice: "" });
  } finally {
    set({ busy: false });
  }
}

export const actions = {
  searchMarket,
  installMarket,
  installJava,
  loadBaseVersion,
  setMarketQuery: (marketQuery: string) => set({ marketQuery }),
  setMarketKind: (marketKind: MarketKind) => set({ marketKind }),
  loadContent,
  toggleMod,
  deleteMod,
  loadLogs,
  openVersionDir,
  setTheme,
  init,
  refresh,
  installSelected,
  launchSelected,
  openDir,
  loadSettings,
  saveSettings,
  loadLoaderOptions,
  loadOptifineOptions,
  setSearch: (search: string) => set({ search }),
  setFilter: (filter: TypeFilter) => set({ filter }),
  addAccount: async (name: string) => {
    try {
      const payload = await invoke<AccountsPayload>("add_account", { name });
      set({ accounts: payload.list, currentAccount: payload.current, notice: "已添加账号 " + name });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  removeAccount: async (id: string) => {
    try {
      const payload = await invoke<AccountsPayload>("remove_account", { id });
      set({ accounts: payload.list, currentAccount: payload.current });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  useAccount: async (id: string) => {
    try {
      const payload = await invoke<AccountsPayload>("use_account", { id });
      set({ accounts: payload.list, currentAccount: payload.current });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  setDefaultVersion: async (id: string | null) => {
    try {
      const cfg = await invoke<AppSettings>("set_default_version", { id });
      set({ defaultVersion: cfg.default_version });
    } catch (e) {
      set({ error: String(e) });
    }
  },
  openVersion: (id: string) => {
    set({ downloadView: { name: "detail", id }, downloadTab: "versions", selected: id });
    void loadBaseVersion(id);
  },
  openVersionSettings: (id: string) => set({ downloadView: { name: "settings", id } }),
  closeVersion: () => set({ downloadView: { name: "list" } }),
  setDownloadTab: (tab: "versions" | "market") => set({ downloadTab: tab }),
  setSelected: (selected: string) => {
    set({ selected });
    void loadSettings(selected);
    void loadContent(selected);
    void loadBaseVersion(selected);
  },
  setLoader: (loader: LoaderKind, mc: string) => void loadLoaderOptions(mc, loader),
  setLoaderVersion: (loaderVersion: string) => set({ loaderVersion }),
  setOptifine: (mc: string, enabled: boolean) => void loadOptifineOptions(mc, enabled),
  setOptifineEdition: (optifineEdition: string) => set({ optifineEdition }),
};
