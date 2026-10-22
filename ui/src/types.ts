export interface VersionSummary {
  id: string;
  kind: string;
  release_time: string;
  installed: boolean;
  /** 只存在于本地 versions/ 目录（别的启动器装的 / 手动放的） */
  local: boolean;
  /** 愚人节快照（清单里仍标 release/snapshot，后端按 id 白名单识别） */
  april_fools: boolean;
  /** 这个版本是 DLP 装的（显示 DLP 徽章） */
  dlp_installed: boolean;
  /** 从版本 id 推断的加载器，原版为 null */
  loader: string | null;
}

/** 离线账号（id 就是离线 UUID） */
/** 微软 device code 登录信息 */
export interface DeviceCode {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface Account {
  id: string;
  name: string;
  /** "offline" 或 "msa" */
  kind?: string;
  skin?: string | null;
  expires_at?: number | null;
}

export interface AccountsPayload {
  current: string | null;
  list: Account[];
}

export interface AppSettings {
  /** "mirror" = BMCLAPI 优先，"official" = 全走官方 */
  download_source: string;
  default_version: string | null;
  curseforge_api_key: string | null;
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
  bytes_done: number;
  bytes_total: number;
  current: string;
  speed: string;
}

/** 一个阶段的进度快照（下载详情页用） */
export interface InstallStage {
  name: string;
  done: number;
  total: number;
  bytes_done: number;
  bytes_total: number;
  percent: number;
  current: string;
  speed: string;
  /** 是不是当前正在跑的那个阶段 */
  active: boolean;
}

export interface InstallState {
  id: string;
  stage: string;
  done: number;
  total: number;
  percent: number;
  /** 已收字节 / 总字节（0 = 这个阶段不按字节算） */
  bytes_done: number;
  bytes_total: number;
  /** 当前正在下载的文件名 */
  current: string;
  /** 速度，形如 "3.2 MB/s" */
  speed: string;
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

export interface BackupEntry {
  name: string;
  world: string;
  time: string;
  bytes: number;
}

export interface ServerEntry {
  name: string;
  ip: string;
  hidden: boolean;
}

/** 整合包摘要（后端 PackPreview） */
export interface PackPreview {
  format: string;
  name: string;
  version: string;
  mc: string;
  /** [加载器键, 版本]，原版为 null */
  loader: [string, string] | null;
  files: number;
  bytes: number;
  overrides: number;
  sample: string[];
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
/** 市场来源 */
export type MarketSource = "modrinth" | "curseforge";

/** 项目详情 */
export interface MarketProject {
  id: string;
  slug: string;
  title: string;
  title_zh: string | null;
  description: string;
  body: string;
  icon: string | null;
  downloads: number;
  categories: string[];
  site_url: string;
  source: string;
}

/** 版本下的一个文件 */
export interface MarketFile {
  version_id: string;
  name: string;
  version_number: string;
  filename: string;
  download_url: string;
  downloads: number;
  game_versions: string[];
  loaders: string[];
  size: number;
  date: string;
}

/** 一个版本（可能挂多个文件） */
export interface MarketVersion {
  version_id: string;
  name: string;
  version_number: string;
  downloads: number;
  game_versions: string[];
  loaders: string[];
  date: string;
  files: MarketFile[];
}

export interface MarketHit {
  id: string;
  slug: string;
  title: string;
  /** 内置词表命中的中文名；没有就显示英文原名 */
  title_zh?: string | null;
  description: string;
  downloads: number;
  icon: string | null;
  source?: string;
}

export interface MarketPage {
  hits: MarketHit[];
  total: number;
  offset: number;
}

export type MarketKind = "mod" | "resourcepack" | "shader";
