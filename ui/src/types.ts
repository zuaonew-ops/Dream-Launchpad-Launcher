export interface VersionSummary {
  id: string;
  kind: string;
  release_time: string;
  installed: boolean;
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
  percent: number;
}
