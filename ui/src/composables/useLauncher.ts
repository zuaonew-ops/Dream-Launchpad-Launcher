import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Environment,
  InstallState,
  JavaInfo,
  ProgressPayload,
  VersionSummary,
} from "../types";

const USERNAME_KEY = "dlp.username";
const MEMORY_KEY = "dlp.memory";
const ISOLATE_KEY = "dlp.isolate";
const JAVA_KEY = "dlp.java";

export type TypeFilter = "release" | "snapshot" | "installed" | "all";

// 全局单一状态：各页面共享，避免每个页面各拉一份
const versions = ref<VersionSummary[]>([]);
const environment = ref<Environment | null>(null);
const javas = ref<JavaInfo[]>([]);
const selected = ref<string | null>(null);
const username = ref(localStorage.getItem(USERNAME_KEY) ?? "Steve");
const memory = ref(Number(localStorage.getItem(MEMORY_KEY) ?? 2048));
const isolate = ref(localStorage.getItem(ISOLATE_KEY) !== "false");
const javaPath = ref(localStorage.getItem(JAVA_KEY) ?? "");
const search = ref("");
const typeFilter = ref<TypeFilter>("release");
const loading = ref(false);
const busy = ref(false);
const error = ref("");
const notice = ref("");
const install = ref<InstallState | null>(null);
const launchPid = ref<number | null>(null);
let subscribed = false;

function groupKey(id: string): string {
  const parts = id.split(".");
  if (parts.length >= 3) return `${parts[0]}.${parts[1]}`;
  if (parts.length === 2) return parts[0] === "1" ? id : parts[0];
  return parts[0];
}

const filtered = computed(() =>
  versions.value.filter((v) => {
    if (typeFilter.value === "installed" && !v.installed) return false;
    if (typeFilter.value === "release" && v.kind !== "release") return false;
    if (typeFilter.value === "snapshot" && v.kind === "release") return false;
    const keyword = search.value.trim().toLowerCase();
    return !keyword || v.id.toLowerCase().includes(keyword);
  }),
);

const groups = computed(() => {
  const order: string[] = [];
  const map = new Map<string, VersionSummary[]>();
  for (const version of filtered.value) {
    const key = groupKey(version.id);
    if (!map.has(key)) {
      map.set(key, []);
      order.push(key);
    }
    map.get(key)!.push(version);
  }
  return order.map((key) => ({ key, items: map.get(key)! }));
});

const latestReleases = computed(() =>
  versions.value.filter((v) => v.kind === "release").slice(0, 4),
);

const selectedVersion = computed(
  () => versions.value.find((v) => v.id === selected.value) ?? null,
);

const installedCount = computed(() => versions.value.filter((v) => v.installed).length);

const activeJava = computed(() => javas.value.find((j) => j.path === javaPath.value) ?? null);

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    const [list, env, found] = await Promise.all([
      invoke<VersionSummary[]>("list_versions", { includeSnapshots: true }),
      invoke<Environment>("environment"),
      invoke<JavaInfo[]>("detect_java"),
    ]);
    versions.value = list;
    environment.value = env;
    javas.value = found;
    if (!selected.value || !list.some((v) => v.id === selected.value)) {
      selected.value =
        list.find((v) => v.kind === "release" && v.installed)?.id ??
        list.find((v) => v.kind === "release")?.id ??
        list[0]?.id ??
        null;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function subscribe() {
  if (subscribed) return () => {};
  subscribed = true;
  const promise = listen<ProgressPayload>("install-progress", (event) => {
    if (!install.value) return;
    const { stage, done, total } = event.payload;
    install.value = {
      ...install.value,
      stage,
      done,
      total,
      percent: total > 0 ? done / total : 0,
    };
  });
  return () => {
    promise.then((off) => off()).catch(() => {});
    subscribed = false;
  };
}

async function installSelected() {
  const target = selectedVersion.value;
  if (!target || busy.value) return;
  busy.value = true;
  error.value = "";
  notice.value = "";
  launchPid.value = null;
  install.value = { id: target.id, stage: "准备中", done: 0, total: 0, percent: 0 };
  try {
    await invoke("install_version", { id: target.id });
    versions.value = versions.value.map((v) =>
      v.id === target.id ? { ...v, installed: true } : v,
    );
    notice.value = `${target.id} 安装完成`;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
    install.value = null;
  }
}

async function launchSelected() {
  const target = selectedVersion.value;
  if (!target || busy.value) return;
  if (!username.value.trim()) {
    error.value = "请先填写离线昵称";
    return;
  }
  busy.value = true;
  error.value = "";
  notice.value = "正在拉起游戏…";
  localStorage.setItem(USERNAME_KEY, username.value);
  localStorage.setItem(MEMORY_KEY, String(memory.value));
  localStorage.setItem(ISOLATE_KEY, String(isolate.value));
  localStorage.setItem(JAVA_KEY, javaPath.value);
  try {
    const pid = await invoke<number>("launch_game", {
      id: target.id,
      username: username.value.trim(),
      memoryMb: memory.value,
      isolate: isolate.value,
      javaPath: javaPath.value || null,
    });
    launchPid.value = pid;
    notice.value = `已启动 ${target.id}，进程 ${pid}`;
  } catch (e) {
    error.value = String(e);
    notice.value = "";
  } finally {
    busy.value = false;
  }
}

async function openDir(kind: "data" | "logs" | "versions") {
  try {
    await invoke("open_dir", { kind });
  } catch (e) {
    error.value = String(e);
  }
}

watch([username, memory, isolate, javaPath], () => {
  localStorage.setItem(USERNAME_KEY, username.value);
  localStorage.setItem(MEMORY_KEY, String(memory.value));
  localStorage.setItem(ISOLATE_KEY, String(isolate.value));
  localStorage.setItem(JAVA_KEY, javaPath.value);
});

export function useLauncher() {
  return {
    versions,
    filtered,
    groups,
    installedCount,
    latestReleases,
    environment,
    javas,
    activeJava,
    selected,
    selectedVersion,
    username,
    memory,
    isolate,
    javaPath,
    search,
    typeFilter,
    loading,
    busy,
    error,
    notice,
    install,
    launchPid,
    refresh,
    subscribe,
    installSelected,
    launchSelected,
    openDir,
  };
}
