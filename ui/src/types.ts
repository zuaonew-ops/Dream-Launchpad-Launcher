export interface VersionSummary {
  id: string;
  kind: string;
  release_time: string;
  installed: boolean;
  /** 只存在于本地 versions/ 目录（别的启动器装的 / 手动放的） */
  local: boolean;
  /** 愚人节快照（清单里仍标 release/snapshot，后端按 id 白名单识别） */
  april_fools: boolean;
  /** 从版本 id 推断的加载器，原版为 null */
  loader: string | null;
}

/** 离线账号（id 就是离线 UUID） */
export interface Account {
  id: string;
  name: string;
}

export interface AccountsPayload {
  current: string | null;
  list: Account[];
}

export interface AppSettings {
  default_version: string | null;
}

/** 下载页内部的三个视图 */
export type DownloadView =
  | { name: "list" }
  | { name: "detail"; id: string }
  | { name: "settings"; id: string };

export interface JavaInfo {
  path: string;
  version: string;
  major: number;
}

export interface Environment {
  os: string;
  arch: string;
  tauri: string;
  core: string;
  data_dir: string;
}

export interface ProgressPayload {
  stage: string;
  done: number;
  total: number;
}

export interface InstallState {
  id: string;
  stage: string;
  done: number;
  total: number;
  percent: number;
}

/** 每个版本独立的启动设置（null = 跟随全局默认） */
export interface VersionSettings {
  memory_mb: number | null;
  java_path: string | null;
  isolate: boolean | null;
  extra_jvm_args: string[];
  /** 额外游戏参数，拼在 main class 之后 */
  game_args: string[];
}

export interface LocalVersion {
  id: string;
  kind: string;
  release_time: string;
  has_json: boolean;
  has_jar: boolean;
  inherits_from: string | null;
}

/** 加载器 / OptiFine 的可选版本 */
export interface Candidate {
  version: string;
  label: string;
  stable: boolean;
}

export type LoaderKind = "none" | "fabric" | "quilt" | "forge" | "neoforge";

export interface ModEntry {
  name: string;
  size: number;
  enabled: boolean;
  jar: boolean;
}

/** 崩溃报告 */
export interface CrashEntry {
  name: string;
  modified: string;
  size: number;
  summary: string;
}

/** 版本概况 */
/** 系统信息：内存滑块的上限按真实内存算 */
export interface SystemInfo {
  total_mb: number;
  available_mb: number;
  cores: number;
}

export interface VersionStats {
  path: string;
  size: number;
  files: number;
  mods: number;
  saves: number;
}

export interface SaveEntry {
  name: string;
  modified: string;
  valid: boolean;
}

export type Theme = "dark" | "light";

/** Modrinth 搜索结果 */
export interface MarketHit {
  id: string;
  slug: string;
  title: string;
  /** 内置词表命中的中文名；没有就显示英文原名 */
  title_zh?: string | null;
  description: string;
  downloads: number;
  icon: string | null;
}

export interface MarketPage {
  hits: MarketHit[];
  total: number;
  offset: number;
}

export type MarketKind = "mod" | "resourcepack" | "shader";
