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
const SNAPSHOT_KEY = "dlp.snapshots";

/** 启动器全部状态与动作，界面组件只负责展示。 */
export function useLauncher() {
  const versions = ref<VersionSummary[]>([]);
  const environment = ref<Environment | null>(null);
  const javas = ref<JavaInfo[]>([]);
  const selected = ref<string | null>(null);
  const username = ref(localStorage.getItem(USERNAME_KEY) ?? "Steve");
  const memory = ref(Number(localStorage.getItem(MEMORY_KEY) ?? 2048));
  const isolate = ref(localStorage.getItem(ISOLATE_KEY) !== "false");
  const javaPath = ref(localStorage.getItem(JAVA_KEY) ?? "");
  const includeSnapshots = ref(localStorage.getItem(SNAPSHOT_KEY) === "true");
  const search = ref("");
  const loading = ref(false);
  const busy = ref(false);
  const error = ref("");
  const notice = ref("");
  const install = ref<InstallState | null>(null);
  const launchPid = ref<number | null>(null);

  const selectedVersion = computed(
    () => versions.value.find((v) => v.id === selected.value) ?? null,
  );

  const filtered = computed(() => {
    const keyword = search.value.trim().toLowerCase();
    if (!keyword) return versions.value;
    return versions.value.filter((v) => v.id.toLowerCase().includes(keyword));
  });

  const activeJava = computed(
    () => javas.value.find((j) => j.path === javaPath.value) ?? null,
  );

  async function refresh() {
    loading.value = true;
    error.value = "";
    try {
      const [list, env, found] = await Promise.all([
        invoke<VersionSummary[]>("list_versions", {
          includeSnapshots: includeSnapshots.value,
        }),
        invoke<Environment>("environment"),
        invoke<JavaInfo[]>("detect_java"),
      ]);
      versions.value = list;
      environment.value = env;
      javas.value = found;
      if (!selected.value || !list.some((v) => v.id === selected.value)) {
        selected.value = list[0]?.id ?? null;
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function subscribe() {
    return listen<ProgressPayload>("install-progress", (event) => {
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

  async function openDataDir() {
    try {
      await invoke("open_data_dir");
    } catch (e) {
      error.value = String(e);
    }
  }

  watch(includeSnapshots, (value) => {
    localStorage.setItem(SNAPSHOT_KEY, String(value));
    refresh();
  });

  return {
    versions,
    filtered,
    environment,
    javas,
    activeJava,
    selected,
    selectedVersion,
    username,
    memory,
    isolate,
    javaPath,
    includeSnapshots,
    search,
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
    openDataDir,
  };
}
