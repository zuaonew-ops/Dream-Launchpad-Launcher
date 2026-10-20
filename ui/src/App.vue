<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import SidePanel from "./components/SidePanel.vue";
import VersionToolbar from "./components/VersionToolbar.vue";
import VersionList from "./components/VersionList.vue";
import ProgressPanel from "./components/ProgressPanel.vue";
import { useLauncher } from "./composables/useLauncher";

const {
  filtered,
  environment,
  javas,
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
} = useLauncher();

let unlisten: (() => void) | undefined;

onMounted(async () => {
  unlisten = await subscribe();
  await refresh();
});

onUnmounted(() => unlisten?.());
</script>

<template>
  <div class="app">
    <header class="top">
      <div class="brand">
        <div class="mark">DLP</div>
        <div class="titles">
          <h1>Dream Launchpad</h1>
          <p class="sub">Minecraft: Java Edition</p>
        </div>
      </div>
      <div v-if="environment" class="env">
        {{ environment.os }} · {{ environment.arch }} · Tauri {{ environment.tauri }} · core
        {{ environment.core }}
      </div>
    </header>

    <main class="body">
      <SidePanel
        v-model:username="username"
        v-model:memory="memory"
        v-model:isolate="isolate"
        v-model:java-path="javaPath"
        :javas="javas"
        :environment="environment"
        @open-dir="openDataDir"
      />

      <section class="panel main">
        <VersionToolbar
          v-model:keyword="search"
          v-model:snapshots="includeSnapshots"
          :loading="loading"
          :total="filtered.length"
          @refresh="refresh"
        />
        <VersionList
          :versions="filtered"
          :selected="selected"
          :loading="loading"
          @select="selected = $event"
        />
      </section>
    </main>

    <footer class="bottom">
      <div class="action">
        <div class="picked">
          <span class="label">当前版本</span>
          <strong>{{ selectedVersion?.id ?? "未选择" }}</strong>
          <span v-if="selectedVersion && !selectedVersion.installed" class="badge">未安装</span>
          <span v-else-if="launchPid" class="badge ok">运行中 · {{ launchPid }}</span>
        </div>

        <div class="progress-slot">
          <ProgressPanel
            v-if="install"
            :stage="install.stage"
            :percent="install.percent"
            :done="install.done"
            :total="install.total"
          />
          <p v-else-if="busy" class="muted-line">正在处理…</p>
        </div>

        <div class="buttons">
          <button
            v-if="selectedVersion && !selectedVersion.installed"
            class="primary"
            :disabled="busy || loading"
            @click="installSelected"
          >
            安装
          </button>
          <button
            v-else
            class="primary"
            :disabled="busy || loading || !selectedVersion?.installed"
            @click="launchSelected"
          >
            启动
          </button>
        </div>
      </div>

      <Transition name="toast">
        <p v-if="error" class="msg error">{{ error }}</p>
        <p v-else-if="notice" class="msg ok">{{ notice }}</p>
      </Transition>
    </footer>
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  display: grid;
  grid-template-rows: auto 1fr auto;
  padding: 20px 24px 16px;
  gap: 14px;
}

.top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  animation: rise 460ms cubic-bezier(0.16, 1, 0.3, 1) both;
}

.brand {
  display: flex;
  align-items: center;
  gap: 14px;
}

.mark {
  width: 44px;
  height: 44px;
  border-radius: 13px;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: #fff;
  background: linear-gradient(135deg, #4c8dff, #22c1a4);
  box-shadow: 0 10px 26px -10px rgba(76, 141, 255, 0.8);
  transition: transform 260ms cubic-bezier(0.2, 0.9, 0.3, 1.4);
}

.mark:hover {
  transform: rotate(-4deg) scale(1.04);
}

h1 {
  font-size: 17px;
  font-weight: 650;
  letter-spacing: 0.2px;
}

.sub {
  font-size: 12px;
  color: var(--muted);
  margin-top: 2px;
}

.env {
  font-size: 12px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.body {
  display: grid;
  grid-template-columns: 292px 1fr;
  gap: 14px;
  min-height: 0;
}

.panel {
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 16px;
}

.main {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  min-height: 0;
  animation: rise 560ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 110ms;
}

.bottom {
  display: flex;
  flex-direction: column;
  gap: 8px;
  animation: rise 600ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 160ms;
}

.action {
  display: grid;
  grid-template-columns: minmax(150px, auto) 1fr auto;
  align-items: center;
  gap: 18px;
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 12px 16px;
}

.picked {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}

.picked strong {
  font-size: 15px;
  font-variant-numeric: tabular-nums;
}

.label {
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.06);
  color: var(--muted);
}

.badge.ok {
  color: #4fd8b8;
  background: rgba(34, 193, 164, 0.12);
}

.progress-slot {
  min-height: 34px;
  display: flex;
  align-items: center;
}

.progress-slot > * {
  width: 100%;
}

.muted-line {
  font-size: 12px;
  color: var(--muted);
}

.buttons {
  display: flex;
  gap: 10px;
}

.primary {
  border: none;
  border-radius: 11px;
  padding: 11px 26px;
  font-size: 14px;
  font-weight: 600;
  color: #fff;
  cursor: pointer;
  background: linear-gradient(135deg, #4c8dff, #3a7bdd);
  box-shadow: 0 10px 24px -12px rgba(76, 141, 255, 0.9);
  transition: transform 160ms cubic-bezier(0.2, 0.9, 0.3, 1.3), box-shadow 160ms ease,
    filter 160ms ease;
}

.primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 14px 30px -12px rgba(76, 141, 255, 1);
}

.primary:active:not(:disabled) {
  transform: translateY(0) scale(0.98);
}

.primary:disabled {
  filter: grayscale(0.5) brightness(0.7);
  cursor: not-allowed;
}

.msg {
  font-size: 12px;
  padding: 2px 4px;
}

.msg.error {
  color: #ff8080;
}

.msg.ok {
  color: #4fd8b8;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 200ms ease, transform 200ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .top,
  .main,
  .bottom {
    animation: none;
  }
}
</style>
