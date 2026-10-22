import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import type {
  BackupEntry,
  ExportResult,
  SkinEntry,
  UpdateInfo,
  DuplicateGroup,
  ShotEntry,
  ServerStatus,
  GameOptions,
  LevelInfo,
  ModUpdate,
  PackPreview,
  ServerEntry,
  InstallStage,
  Account,
  AccountsPayload,
  AppSettings,
  Candidate,
  CrashEntry,
  DeviceCode,
  SystemInfo,
  VersionStats,
  DownloadView,
  Environment,
  InstallState,
  JavaInfo,
  LoaderKind,
  ProgressPayload,
  MarketHit,
  MarketKind,
  MarketPage,
  MarketProject,
  MarketSource,
  MarketVersion,
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
  downloadTab: "versions" | "market" | "pack";
  /** 选中的整合包 zip 绝对路径 */
  packFile: string;
  /** 整合包摘要（预览） */
  packPreview: PackPreview | null;
  /** 该版本的存档备份列表 */
  backups: BackupEntry[];
  /** 该版本的服务器列表（servers.dat） */
  servers: ServerEntry[];
  /** 应用级设置（下载源、CurseForge key、默认版本） */
  appSettings: AppSettings | null;
  /** 皮肤库 */
  skins: SkinEntry[];
  /** 这个版本装没装 CustomSkinLoader */
  hasCsl: boolean;
  /** 检测到的可更新模组 */
  modUpdates: ModUpdate[];
  /** 启动器自身更新信息 */
  launcherUpdate: UpdateInfo | null;
  /** 装了多个版本的模组 */
  modDuplicates: DuplicateGroup[];
  /** 游戏截图 */
  shots: ShotEntry[];
  /** 服务器在线状态（按 host:port 索引） */
  serverStatus: Record<string, ServerStatus>;
  /** 游戏内设置（options.txt） */
  gameOptions: GameOptions | null;
  /** 展开的存档详情（按世界名索引） */
  levelInfo: Record<string, LevelInfo | null>;
  /** 装出来的版本 id（可手改） */
  packId: string;
  search: string;
  filter: TypeFilter;
  loading: boolean;
  busy: boolean;
  error: string;
  notice: string;
  install: InstallState | null;
  /** 每个阶段各自的进度（下载详情页用） */
  installStages: InstallStage[];
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
  /** 资源搜索的三个筛选：先选版本，再选加载器，最后类别 */
  marketMc: string;
  marketLoader: string;
  marketCategory: string;
  /** 市场来源 */
  marketSource: MarketSource;
  /** 打开的项目详情；null = 停留在列表 */
  marketOpen: MarketProject | null;
  marketVersions: MarketVersion[];
  marketVersionsLoading: boolean;
  /** 详情里展开的版本 id */
  marketExpanded: string | null;
  /** 详情页是否只看当前版本 + 加载器适配的版本 */
  onlyCompatible: boolean;

  // 内容管理
  mods: ModEntry[];
  saves: SaveEntry[];
  /** 资源包 / 光影（按需加载） */
  packs: { resourcepacks: ModEntry[]; shaderpacks: ModEntry[] };
  logs: string[];
  logName: string;
  logText: string;
  /** 崩溃报告（crash-reports/*.txt） */
  crashes: CrashEntry[];
  /** 当前查看版本的概况（大小/文件数/模组数） */
  versionStats: VersionStats | null;
  /** 物理内存总量等，内存滑块的量程靠它 */
  system: SystemInfo | null;
  /** 正版登录流程（全局，切页面不中断） */
  msa: MsaLogin | null;
  /** 安装前的确认弹窗：装什么、装到哪 */
  installAsk: InstallAsk | null;
  /** 弹窗里选的目标版本 */
  installTargetId: string;
  /** 装完的提示文案（弹窗内显示） */
  installDone: string;
  /** 市场当前页码（1 起） */
  marketPage: number;
  /** 正在启动的游戏：步骤 + 进度，null = 没在启动 */
  launching: LaunchProgress | null;
  crashName: string;
  crashText: string;
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
    game_args: [],
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
  packFile: "",
  packPreview: null,
  backups: [],
  servers: [],
  appSettings: null,
  skins: [],
  hasCsl: false,
  modUpdates: [],
  launcherUpdate: null,
  modDuplicates: [],
  shots: [],
  serverStatus: {},
  gameOptions: null,
  levelInfo: {},
  packId: "",
  search: "",
  filter: "release",
  loading: false,
  busy: false,
  error: "",
  notice: "",
  install: null,
  installStages: [],
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
  marketMc: "",
  marketLoader: "any",
  marketCategory: "all",
  marketSource: "modrinth",
  marketOpen: null,
  marketVersions: [],
  marketVersionsLoading: false,
  marketExpanded: null,
  onlyCompatible: true,
  mods: [],
  saves: [],
  packs: { resourcepacks: [], shaderpacks: [] },
  logs: [],
  logName: "",
  logText: "",
  crashes: [],
  versionStats: null,
  system: null,
  msa: null,
  installAsk: null,
  installTargetId: "",
  installDone: "",
  marketPage: 1,
  launching: null,
  crashName: "",
  crashText: "",
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
      appSettings: cfg,
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
    installStages: [],
    install: {
      id: target.id,
      stage: "准备中",
      done: 0,
      total: 0,
      percent: 0,
      bytes_done: 0,
      bytes_total: 0,
      current: "",
      speed: "",
    },
  });

  let finalId = target.id;
  try {
    await invoke("install_version", { id: target.id });
    if (state.loader !== "none" && state.loaderVersion) {
      set({
        install: { ...state.install!, stage: `安装 ${state.loader}`, percent: 0, current: "" },
      });
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
  set({
    busy: true,
    error: "",
    notice: "",
    launching: { id: target.id, step: 0, percent: 0.06, error: "" },
  });

  // 后端的启动是一次调用，这里用定时器把「阶段」推进到 0.75 左右，
  // 真正返回 pid 再补到 100% —— 用户能看到走到哪一步，而不是干等。
  const timer = window.setInterval(() => {
    const current = state.launching;
    if (!current) return;
    const next = Math.min(current.percent + 0.18, 0.75);
    const step = Math.min(Math.floor(next * LAUNCH_STEPS.length), LAUNCH_STEPS.length - 1);
    set({ launching: { ...current, percent: next, step } });
  }, 420);

  try {
    const pid = await invoke<number>("launch_game", {
      id: target.id,
      username: name,
    });
    window.clearInterval(timer);
    set({
      launching: { id: target.id, step: LAUNCH_STEPS.length, percent: 1, error: "" },
      game: { pid, id: target.id },
      notice: `已启动 ${target.id}，进程 ${pid}`,
    });
    // 让「完成」状态停留一下再收起来
    window.setTimeout(() => {
      if (state.launching?.percent === 1) set({ launching: null });
    }, 900);
  } catch (e) {
    window.clearInterval(timer);
    set({
      error: String(e),
      launching: { id: target.id, step: 0, percent: 0, error: String(e) },
    });
  } finally {
    set({ busy: false });
  }
}

function dismissLaunch() {
  set({ launching: null });
}

/** 打开 MC 百科：先查出该模组的项目页，查不到退回搜索页 */
async function openMcmod(title: string) {
  set({ busy: true, error: "" });
  try {
    const url = await invoke<string | null>("mcmod_url", { title });
    const target = url ?? `https://search.mcmod.cn/s?key=${encodeURIComponent(title)}`;
    await invoke("open_url", { url: target });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ busy: false });
  }
}

/** 用系统浏览器打开 https 链接（后端用 ShellExecuteW，不会闪 cmd 窗口） */
async function openUrl(url: string) {
  try {
    await invoke("open_url", { url });
  } catch (e) {
    set({ error: String(e) });
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
    const payload = event.payload;
    if (!state.install) return;
    // 有字节数就按字节算（大文件才不会长时间停在同一个百分比）
    const byBytes = payload.bytes_total > 0;
    const percent = byBytes
      ? Math.min(1, payload.bytes_done / payload.bytes_total)
      : payload.total > 0
        ? payload.done / payload.total
        : 0;
    const stagePercent = byBytes
      ? Math.min(1, payload.bytes_done / payload.bytes_total)
      : payload.total > 0
        ? Math.min(1, payload.done / payload.total)
        : 0;

    const next: InstallStage = {
      name: payload.stage,
      done: payload.done,
      total: payload.total,
      bytes_done: payload.bytes_done,
      bytes_total: payload.bytes_total,
      percent: stagePercent,
      current: payload.current,
      speed: payload.speed,
      active: true,
    };
    const stages = [...state.installStages];
    const index = stages.findIndex((item) => item.name === payload.stage);
    if (index >= 0) stages[index] = next;
    else stages.push(next);

    set({
      install: {
        ...state.install,
        stage: payload.stage,
        done: payload.done,
        total: payload.total,
        bytes_done: payload.bytes_done,
        bytes_total: payload.bytes_total,
        current: payload.current,
        speed: payload.speed,
        percent,
      },
      installStages: stages.map((item) => ({ ...item, active: item.name === payload.stage })),
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

  void loadSystem();

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

/** 拉资源包 / 光影列表 */
async function loadPacks(id: string, kind: "resourcepacks" | "shaderpacks") {
  try {
    const list = await invoke<ModEntry[]>("list_packs", { id, kind });
    set({ packs: { ...state.packs, [kind]: list } });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 删除一个资源包 / 光影 */
async function deletePack(id: string, kind: "resourcepacks" | "shaderpacks", name: string) {
  try {
    await invoke("delete_pack", { id, kind, name });
    await loadPacks(id, kind);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadContent(id: string) {
  try {
    const [mods, saves, backups] = await Promise.all([
      invoke<ModEntry[]>("list_mods", { id }),
      invoke<SaveEntry[]>("list_saves", { id }),
      invoke<BackupEntry[]>("list_backups", { id }),
    ]);
    set({ mods, saves, backups });
    void loadModDuplicates(id);
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

/** 校验并修复：重跑一遍安装，已完整的文件会跳过，缺的会重下 */
async function repairVersion(id: string) {
  if (state.busy) return;
  set({
    busy: true,
    error: "",
    notice: "",
    installStages: [],
    install: {
      id,
      stage: "校验文件",
      done: 0,
      total: 0,
      percent: 0,
      bytes_done: 0,
      bytes_total: 0,
      current: "",
      speed: "",
    },
  });
  try {
    await invoke("repair_version", { id });
    await refresh();
    set({ notice: `${id} 校验完成` });
  } catch (e) {
    set({ error: String(e) });
  } finally {
    set({ busy: false, install: null });
  }
}

async function deleteVersion(id: string) {
  try {
    await invoke("delete_version", { id });
    if (state.selected === id) set({ selected: null });
    if (state.downloadView.name !== "list") set({ downloadView: { name: "list" } });
    await refresh();
    set({ notice: `已删除 ${id}` });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function setAllMods(id: string, enabled: boolean) {
  try {
    const changed = await invoke<number>("all_mods", { id, enabled });
    await loadContent(id);
    set({ notice: enabled ? `已启用 ${changed} 个模组` : `已禁用 ${changed} 个模组` });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 拉到某版本的概况；切版本时先清空，避免显示上一个版本的数字 */
/** 建议上限：留 1G 给系统，最多给到物理内存的 80%，向上取整到 256MB */
/** 安装确认：三种来源共用（列表安装 / 详情文件下载） */
/** 每页拉多少条 */
const PAGE_SIZE = 20;

export interface InstallAsk {
  title: string;
  kind: MarketKind;
  /** Modrinth / CurseForge 的项目 id 或 slug（列表安装用） */
  project?: string;
  /** 直接下载某个文件的地址（详情页文件用） */
  url?: string;
  filename?: string;
}

export interface LaunchProgress {
  /** 正在启动的版本 */
  id: string;
  /** 已经走到的步骤下标 */
  step: number;
  /** 0–1 */
  percent: number;
  /** 出错了就停在这里 */
  error: string;
}

/** 启动步骤，界面按这个列表逐条点亮 */
export const LAUNCH_STEPS = [
  "检查 Java 运行时",
  "校验版本文件",
  "组装启动参数",
  "拉起游戏进程",
] as const;

export interface MsaLogin {
  code: DeviceCode | null;
  status: "requesting" | "waiting" | "done" | "error";
  message: string;
}

let msaTimer: number | null = null;

function stopMsaTimer() {
  if (msaTimer !== null) {
    window.clearInterval(msaTimer);
    msaTimer = null;
  }
}

/** 开始正版登录：申请设备码 → 自动打开浏览器 → 轮询 */
async function startMsaLogin() {
  // 已经在走流程就别再申请一次（否则会连着开好几个浏览器窗口）
  const busy = state.msa && (state.msa.status === "requesting" || state.msa.status === "waiting");
  if (busy) return;
  stopMsaTimer();
  set({
    msa: { code: null, status: "requesting", message: "正在向微软申请设备码…" },
    error: "",
  });

  let code: DeviceCode | null = null;
  try {
    code = await invoke<DeviceCode>("msa_start");
  } catch (e) {
    set({ msa: { code: null, status: "error", message: String(e) } });
    return;
  }
  if (!code) {
    set({ msa: { code: null, status: "error", message: state.error || "申请设备码失败" } });
    return;
  }

  set({ msa: { code, status: "waiting", message: "等待浏览器里确认…" } });

  // 直接把浏览器拉起来，省得用户手动复制链接
  void invoke("open_url", { url: code.verification_uri }).catch(() => undefined);

  const deadline = Date.now() + code.expires_in * 1000;
  msaTimer = window.setInterval(async () => {
    if (Date.now() > deadline) {
      stopMsaTimer();
      set({ msa: { code, status: "error", message: "这个码过期了，重新点一次「正版登录」" } });
      return;
    }
    try {
      const account = await invoke<Account | null>("msa_poll", { deviceCode: code!.device_code });
      if (account) {
        stopMsaTimer();
        const payload = await invoke<AccountsPayload>("accounts");
        set({
          accounts: payload.list,
          currentAccount: payload.current,
          msa: { code, status: "done", message: `已登录 ${account.name}` },
        });
      }
    } catch (e) {
      stopMsaTimer();
      set({ msa: { code, status: "error", message: String(e) } });
    }
  }, Math.max(code.interval, 3) * 1000);
}

function closeMsaLogin() {
  stopMsaTimer();
  set({ msa: null });
}

export function memoryCeiling(totalMb: number): number {
  const eighty = Math.floor((totalMb * 8) / 10);
  const capped = Math.max(totalMb - 1024, 512);
  const value = Math.min(eighty, capped);
  return Math.max(512, Math.floor(value / 256) * 256);
}

/** 第一步：拿设备码，界面要把它显示给用户 */

/** 第二步：轮询。null 表示用户还没在浏览器里点同意 */

async function loadSystem() {
  try {
    const system = await invoke<SystemInfo>("system_info");
    set({ system });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadVersionStats(id: string) {
  set({ versionStats: null });
  try {
    const versionStats = await invoke<VersionStats>("version_stats", { id });
    set({ versionStats });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadCrashes() {
  try {
    const crashes = await invoke<CrashEntry[]>("crash_reports");
    set({ crashes });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function readCrash(name: string) {
  try {
    const crashText = await invoke<string>("read_crash", { name });
    set({ crashName: name, crashText });
  } catch (e) {
    set({ error: String(e) });
  }
}

type VersionDirKind = "root" | "mods" | "saves" | "instance" | "resourcepacks" | "shaderpacks";

async function openVersionDir(id: string, kind: VersionDirKind) {
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

async function searchMarket(page = 1) {
  // 「不指定」就是不按版本过滤 —— 不要偷偷回退到当前版本，否则结果跟筛了版本一样
  const mc = state.marketMc;
  const target = Math.max(1, page);
  const offset = (target - 1) * PAGE_SIZE;
  set({ marketLoading: true, error: "", marketOpen: null, marketHits: [] });
  try {
    const result = await invoke<MarketPage>("search_market", {
      query: state.marketQuery,
      kind: state.marketKind,
      mc: mc || null,
      loader: state.marketLoader === "any" ? null : state.marketLoader,
      category: state.marketCategory === "all" ? null : state.marketCategory,
      source: state.marketSource,
      offset,
      limit: PAGE_SIZE,
    });
    set({
      marketHits: result.hits,
      marketTotal: result.total,
      marketPage: target,
      marketLoading: false,
    });
  } catch (e) {
    set({ error: String(e), marketHits: [], marketTotal: 0, marketLoading: false });
  }
}

/** 翻到第几页 */
function goMarketPage(page: number) {
  if (state.marketLoading) return;
  void searchMarket(page);
}

/** 点开一个项目：拉详情 + 该版本/加载器下可用文件 */
async function openMarketProject(hit: MarketHit) {
  set({
    marketOpen: {
      id: hit.id,
      slug: hit.slug,
      title: hit.title,
      title_zh: hit.title_zh ?? null,
      description: hit.description,
      body: "",
      icon: hit.icon,
      downloads: hit.downloads,
      categories: [],
      site_url: "",
      source: hit.source ?? state.marketSource,
    },
    marketVersions: [],
    marketVersionsLoading: true,
    marketExpanded: null,
  onlyCompatible: true,
  });

  try {
    const project = await invoke<MarketProject>("market_project", {
      project: hit.slug,
      source: state.marketSource,
    });
    set({ marketOpen: project });
    await loadMarketVersions();
  } catch (e) {
    set({ error: String(e), marketVersionsLoading: false });
  }
}

/** 拉详情页的版本列表：onlyCompatible 关掉时拿全部版本 */
async function loadMarketVersions() {
  const project = state.marketOpen;
  if (!project) return;
  const mc = state.marketMc;
  const loader = state.marketLoader === "any" ? null : state.marketLoader;
  set({ marketVersionsLoading: true });
  try {
    const versions = await invoke<MarketVersion[]>("market_versions", {
      project: project.slug,
      mc: state.onlyCompatible ? mc || null : null,
      loader: state.onlyCompatible ? loader : null,
      source: project.source ?? state.marketSource,
    });
    set({ marketVersions: versions, marketExpanded: versions[0]?.version_id ?? null });
  } catch (e) {
    set({ error: String(e) });
    set({ marketVersions: [] });
  } finally {
    set({ marketVersionsLoading: false });
  }
}

function closeMarketProject() {
  set({ marketOpen: null, marketVersions: [], marketExpanded: null });
}

function toggleMarketVersion(id: string) {
  set({ marketExpanded: state.marketExpanded === id ? null : id });
}

/** 点「下载」——先弹确认，让用户挑装到哪个版本的 mods 目录 */
function installMarketFile(url: string, filename: string) {
  askInstall({ title: filename, kind: state.marketKind, url, filename });
}

/** 点「安装」——同样先确认 */
function installMarket(project: string, kind: MarketKind) {
  askInstall({ title: project, kind, project });
}

/** 打开安装确认弹窗 */
function askInstall(ask: InstallAsk) {
  set({
    installAsk: ask,
    installTargetId: state.marketMc || state.selected || "",
    installDone: "",
    error: "",
  });
}

function cancelInstall() {
  set({ installAsk: null, install: null, installDone: "" });
}

function setInstallTargetId(installTargetId: string) {
  set({ installTargetId });
}

/** 确认安装：后端会推 install-progress，弹窗里就能看到进度条 */
async function confirmInstall() {
  const ask = state.installAsk;
  const id = state.installTargetId;
  if (!ask) return;
  if (!id) {
    set({ error: "先选一个已安装的版本作为安装目标" });
    return;
  }

  set({
    busy: true,
    error: "",
    installDone: "",
    install: {
      id,
      stage: "准备中",
      done: 0,
      total: 0,
      percent: 0,
      bytes_done: 0,
      bytes_total: 0,
      current: "",
      speed: "",
    },
  });

  try {
    const path = await invoke<string>(ask.url ? "install_market_file" : "install_market", {
      id,
      kind: ask.kind,
      ...(ask.url ? { url: ask.url, filename: ask.filename } : { project: ask.project }),
    });
    set({
      busy: false,
      install: {
      id,
      stage: "完成",
      done: 1,
      total: 1,
      percent: 1,
      bytes_done: 0,
      bytes_total: 0,
      current: "",
      speed: "",
    },
      installDone: path,
      notice: `已安装 ${path}`,
    });
    await loadContent(id);
  } catch (e) {
    set({ busy: false, error: String(e), install: null });
  }
}

/** 存档详情：点一下才拉，列表里不用一次读全部 level.dat */
async function loadLevelInfo(id: string, world: string) {
  if (state.levelInfo[world]) {
    // 已经拉过就折叠
    set({ levelInfo: { ...state.levelInfo, [world]: null } });
    return;
  }
  try {
    const info = await invoke<LevelInfo | null>("level_info", { id, world });
    set({ levelInfo: { ...state.levelInfo, [world]: info } });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 游戏内设置 */
async function loadGameOptions(id: string) {
  try {
    const gameOptions = await invoke<GameOptions>("game_options", { id });
    set({ gameOptions });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function saveGameOptions(id: string, options: GameOptions) {
  set({ gameOptions: options });
  try {
    await invoke("save_game_options", { id, options });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 查所有服务器的在线状态 */
async function pingServers(id: string) {
  try {
    set({ serverStatus: {} });
    const list = await invoke<ServerStatus[]>("ping_servers", { id });
    const map: Record<string, ServerStatus> = {};
    for (const item of list) map[`${item.host}:${item.port}`] = item;
    set({ serverStatus: map });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 截图 */
async function loadShots(id: string) {
  try {
    const shots = await invoke<ShotEntry[]>("list_shots", { id });
    set({ shots });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function openShot(id: string, name: string) {
  try {
    await invoke("open_shot", { id, name });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function deleteShot(id: string, name: string) {
  try {
    await invoke("delete_shot", { id, name });
    await loadShots(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 查重复模组 */
async function loadModDuplicates(id: string) {
  try {
    const modDuplicates = await invoke<DuplicateGroup[]>("mod_duplicates", { id });
    set({ modDuplicates });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 启动时查一次启动器更新（失败静默，不打扰） */
async function checkLauncherUpdate() {
  try {
    const info = await invoke<UpdateInfo | null>("check_launcher_update");
    if (info?.newer) set({ launcherUpdate: info });
  } catch {
    // 没网 / 没发过版：安静地跳过
  }
}

/** 检查模组更新（按 SHA1 问 Modrinth） */
async function checkModUpdates(id: string) {
  set({ busy: true, error: "", notice: "" });
  try {
    const updates = await invoke<ModUpdate[]>("check_mod_updates", { id });
    set({
      busy: false,
      modUpdates: updates,
      notice:
        updates.length === 0
          ? "所有模组都是最新的"
          : `${updates.length} 个模组有新版本`,
    });
  } catch (e) {
    set({ busy: false, error: String(e) });
  }
}

/** 更新一个模组 */
async function applyModUpdate(id: string, update: ModUpdate) {
  try {
    await invoke("update_mod", {
      id,
      oldFile: update.file_name,
      url: update.download_url,
      filename: update.latest_file,
    });
    set({
      modUpdates: state.modUpdates.filter((m) => m.file_name !== update.file_name),
      notice: `${update.title} 已更新到 ${update.latest}`,
    });
    await loadContent(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 复制一个版本 */
async function cloneVersion(from: string) {
  const to = window.prompt(`把 ${from} 复制成：`, `${from}-副本`);
  if (!to || to === from) return;
  try {
    await invoke("clone_version", { from, to });
    await refresh();
    set({ notice: `已复制成 ${to}` });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 版本改名 */
async function renameVersion(from: string) {
  const to = window.prompt(`把 ${from} 改名为（会重建目录）：`, from);
  if (!to || to === from) return;
  try {
    await invoke("rename_version", { from, to });
    await refresh();
    set({ selected: to, notice: `已改名为 ${to}` });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 皮肤库 + 是否装了 CustomSkinLoader */
async function loadSkins(id: string) {
  try {
    const [skins, hasCsl] = await Promise.all([
      invoke<SkinEntry[]>("list_skins", { id }),
      invoke<boolean>("has_customskinloader", { id }),
    ]);
    set({ skins, hasCsl });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function importSkin(id: string) {
  const name = window.prompt("给这个皮肤起个名字（不用写 .png）", "skin");
  if (!name) return;
  try {
    const saved = await invoke<string>("import_skin", { id, name });
    set({ notice: `已导入皮肤 ${saved}` });
    await loadSkins(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function deleteSkin(id: string, name: string) {
  try {
    await invoke("delete_skin", { id, name });
    await loadSkins(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function applySkin(id: string, skin: string, player: string) {
  try {
    const placed = await invoke<string>("apply_skin", { id, skin, player });
    set({ notice: `皮肤已放到 ${placed}` });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 把一个版本导出成 .mrpack */
async function exportPack(id: string) {
  try {
    const path = await invoke<string | null>("pick_export_path", {
      defaultName: `${id}.mrpack`,
    });
    if (!path) return;
    set({ busy: true, error: "", notice: "" });
    const result = await invoke<ExportResult>("export_pack", { id, path });
    set({
      busy: false,
      notice: `已导出：${result.files} 个文件，${result.path}`,
    });
  } catch (e) {
    set({ busy: false, error: String(e) });
  }
}

/** 保存应用级设置（下载源 / CurseForge key / 默认版本） */
async function saveAppSettings(patch: Partial<AppSettings>) {
  const current = state.appSettings;
  const next: AppSettings = {
    default_version: patch.default_version ?? current?.default_version ?? null,
    curseforge_api_key: patch.curseforge_api_key ?? current?.curseforge_api_key ?? null,
    download_source: patch.download_source ?? current?.download_source ?? "mirror",
  };
  try {
    const saved = await invoke<AppSettings>("save_app_settings", { settings: next });
    set({ appSettings: saved, notice: "设置已保存" });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 备份一个存档 */
async function backupSave(id: string, world: string) {
  try {
    const name = await invoke<string>("backup_save", { id, world });
    set({ notice: `已备份 ${world} → ${name}` });
    await loadBackups(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function loadBackups(id: string) {
  try {
    const backups = await invoke<BackupEntry[]>("list_backups", { id });
    set({ backups });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function deleteBackup(id: string, name: string) {
  try {
    await invoke("delete_backup", { id, name });
    await loadBackups(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

async function restoreBackup(id: string, name: string) {
  try {
    const world = await invoke<string>("restore_backup", { id, name });
    set({ notice: `已还原存档 ${world}` });
    await loadContent(id);
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 服务器列表（读写 servers.dat） */
async function loadServers(id: string) {
  try {
    const servers = await invoke<ServerEntry[]>("list_servers", { id });
    set({ servers });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function addServer(id: string, name: string, ip: string) {
  try {
    const servers = await invoke<ServerEntry[]>("add_server", { id, name, ip });
    set({ servers, notice: `已添加服务器 ${name}` });
  } catch (e) {
    set({ error: String(e) });
  }
}

async function removeServer(id: string, ip: string) {
  try {
    const servers = await invoke<ServerEntry[]>("remove_server", { id, ip });
    set({ servers });
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 选一个本地整合包文件 */
async function pickPack() {
  try {
    const path = await invoke<string | null>("pick_pack");
    if (!path) return;
    set({ packFile: path, packPreview: null, packId: "", error: "", notice: "" });
    await loadPackPreview(path);
  } catch (e) {
    set({ error: String(e) });
  }
}

/** 读整合包摘要 */
async function loadPackPreview(path: string) {
  try {
    const preview = await invoke<PackPreview>("pack_preview", { path });
    set({ packPreview: preview, packId: preview.name });
  } catch (e) {
    set({ packPreview: null, error: String(e) });
  }
}

/** 安装整合包：后端会装依赖版本、下文件、覆盖 overrides */
async function installPack() {
  const path = state.packFile;
  if (!path || state.busy) return;
  set({
    busy: true,
    error: "",
    notice: "",
    install: {
      id: state.packId || "整合包",
      stage: "准备安装整合包",
      done: 0,
      total: 0,
      percent: 0,
      bytes_done: 0,
      bytes_total: 0,
      current: "",
      speed: "",
    },
  });
  try {
    const id = await invoke<string>("install_pack", { path, id: state.packId });
    await refresh();
    set({
      busy: false,
      notice: `整合包已装成版本 ${id}`,
      selected: id,
      install: null,
      downloadView: { name: "detail", id },
    });
  } catch (e) {
    set({ busy: false, install: null, error: String(e) });
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
  askInstall,
  cancelInstall,
  setInstallTargetId,
  confirmInstall,
  goMarketPage,
  openMarketProject,
  closeMarketProject,
  toggleMarketVersion,
  installMarketFile,
  installJava,
  loadBaseVersion,
  setMarketQuery: (marketQuery: string) => set({ marketQuery, marketPage: 1 }),
  setMarketSource: (source: MarketSource) => set({ marketSource: source, marketOpen: null }),
  setMarketKind: (marketKind: MarketKind) => set({ marketKind, marketCategory: "all" }),
  setMarketMc: (marketMc: string) => set({ marketMc }),
  setOnlyCompatible: (onlyCompatible: boolean) => {
    set({ onlyCompatible, marketExpanded: "" });
    void loadMarketVersions();
  },
  setMarketLoader: (marketLoader: string) => set({ marketLoader }),
  setMarketCategory: (marketCategory: string) => set({ marketCategory }),
  loadContent,
  loadPacks,
  deletePack,
  toggleMod,
  deleteMod,
  loadLogs,
  openVersionDir,
  setTheme,
  init,
  refresh,
  installSelected,
  launchSelected,
  dismissLaunch,
  /** 跳到下载任务详情页 */
  goTaskPage: () => window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "task" })),
  /** 从任务页回下载页 */
  backToDownload: () => window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" })),
  openDir,
  openUrl,
  openMcmod,
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
    // 视图在下载页里渲染，所以从主页点进来必须先切过去，否则「点了没反应」
    window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "download" }));
  },
  /** 打开某个版本的设置：独立页面，侧边栏不高亮任何项 */
  openVersionSettings: (id: string) => {
    set({ selected: id, settingsKey: id, downloadView: { name: "list" } });
    void loadBaseVersion(id);
    void loadSettings(id);
    window.dispatchEvent(new CustomEvent("dlp:nav", { detail: "version" }));
  },
  /** 侧边栏「设置」= 全局设置 */
  openGlobalSettings: () => {
    set({ settingsKey: "*" });
    void loadSettings("*");
  },
  closeVersion: () => set({ downloadView: { name: "list" } }),
  repairVersion,
  deleteVersion,
  loadVersionStats,
  loadSystem,
  startMsaLogin,
  closeMsaLogin,
  setAllMods,
  loadCrashes,
  readCrash,
  setDownloadTab: (tab: "versions" | "market" | "pack") => set({ downloadTab: tab }),
  setPackId: (packId: string) => set({ packId }),
  saveAppSettings,
  exportPack,
  cloneVersion,
  loadLevelInfo,
  loadGameOptions,
  saveGameOptions,
  pingServers,
  loadShots,
  openShot,
  deleteShot,
  loadModDuplicates,
  checkLauncherUpdate,
  checkModUpdates,
  applyModUpdate,
  renameVersion,
  loadSkins,
  importSkin,
  deleteSkin,
  applySkin,
  setDownloadSource: (source: "mirror" | "official") => void saveAppSettings({ download_source: source }),
  setCurseForgeKey: (key: string) => void saveAppSettings({ curseforge_api_key: key }),
  pickPack,
  loadPackPreview,
  installPack,
  backupSave,
  deleteBackup,
  restoreBackup,
  loadServers,
  addServer,
  removeServer,
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
