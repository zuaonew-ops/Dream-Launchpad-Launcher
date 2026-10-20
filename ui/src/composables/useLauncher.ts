import { computed, ref } from "vue";
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

/** 启动器全部状态与动作，界面组件只负责展示。 */
export function useLauncher() {
  const versions = ref<VersionSummary[]>([]);
  const environment = ref<Environment | null>(null);
  const javas = ref<JavaInfo[]>([]);
  const selected = ref<string | null>(null);
  const username = ref(localStorage.getItem(USERNAME_KEY) ?? "Steve");
  const memory = ref(Number(localStorage.getItem(MEMORY_KEY) ?? 2048));
  const isolate = ref(localStorage.getItem(ISOLATE_KEY) !== "false");
  const loading = ref(false);
  const busy = ref(false);
  const error = ref("");
  const notice = ref("");
  const install = ref<InstallState | null>(null);

  const selectedVersion = computed(
    () => versions.value.find((v) => v.id === selected.value) ?? null,
  );

  async function refresh() {
    loading.value = true;
    error.value = "";
    try {
      const [list, env, found] = await Promise.all([
        invoke<VersionSummary[]>("list_versions", { includeSnapshots: false }),
        invoke<Environment>("environment"),
        invoke<JavaInfo[]>("detect_java"),
      ]);
      versions.value = list;
      environment.value = env;
      javas.value = found;
      if (!selected.value || !list.some((v) => v.id === selected.value)) {
        selected.value = list.find((v) => v.id === env.core)?.id ?? list[0]?.id ?? null;
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function onProgress(payload: ProgressPayload) {
    if (!install.value) return;
    const percent = payload.total > 0 ? payload.done / payload.total : 0;
    install.value = { ...install.value, stage: payload.stage, percent };
  }

  function subscribe() {
    return listen<ProgressPayload>("install-progress", (event) => onProgress(event.payload));
  }

  async function installSelected() {
    const target = selectedVersion.value;
    if (!target || busy.value) return;
    busy.value = true;
    error.value = "";
    notice.value = "";
    install.value = { id: target.id, stage: "准备中", percent: 0 };
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
    notice.value = "";
    localStorage.setItem(USERNAME_KEY, username.value);
    localStorage.setItem(MEMORY_KEY, String(memory.value));
    localStorage.setItem(ISOLATE_KEY, String(isolate.value));
    try {
      await invoke<number>("launch_game", {
        id: target.id,
        username: username.value.trim(),
        memoryMb: memory.value,
        isolate: isolate.value,
      });
      notice.value = `已启动 ${target.id}（pid 见日志）`;
    } catch (e) {
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  }

  return {
    versions,
    environment,
    javas,
    selected,
    selectedVersion,
    username,
    memory,
    isolate,
    loading,
    busy,
    error,
    notice,
    install,
    refresh,
    subscribe,
    installSelected,
    launchSelected,
  };
}
