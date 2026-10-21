export interface VersionSummary {
  id: string;
  kind: string;
  release_time: string;
  installed: boolean;
  /** 只存在于本地 versions/ 目录（别的启动器装的 / 手动放的） */
  local: boolean;
}

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
