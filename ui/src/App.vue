<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import DownloadPage from "./pages/DownloadPage.vue";
import HomePage from "./pages/HomePage.vue";
import SettingsPage from "./pages/SettingsPage.vue";
import ToolsPage from "./pages/ToolsPage.vue";
import { useLauncher } from "./composables/useLauncher";

type PageKey = "home" | "download" | "settings" | "tools";

const { environment, refresh, subscribe } = useLauncher();
const page = ref<PageKey>("home");

const pages = {
  home: HomePage,
  download: DownloadPage,
  settings: SettingsPage,
  tools: ToolsPage,
} as const;

const current = computed(() => pages[page.value]);

const nav: { key: PageKey; label: string }[] = [
  { key: "home", label: "主页" },
  { key: "download", label: "下载" },
  { key: "settings", label: "设置" },
  { key: "tools", label: "工具" },
];

let unlisten: (() => void) | undefined;
onMounted(async () => {
  unlisten = subscribe();
  await refresh();
});
onUnmounted(() => unlisten?.());
</script>

<template>
  <div class="app">
    <nav class="rail">
      <div class="logo">DLP</div>

      <div class="nav">
        <button
          v-for="item in nav"
          :key="item.key"
          class="nav-item"
          :class="{ active: page === item.key }"
          @click="page = item.key"
        >
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <template v-if="item.key === 'home'">
              <path d="M4 11 12 4l8 7v9h-6v-6h-4v6H4z" />
            </template>
            <template v-else-if="item.key === 'download'">
              <path d="M12 4v11m0 0 4-4m-4 4-4-4M5 19h14" />
            </template>
            <template v-else-if="item.key === 'settings'">
              <path d="M5 8h14M5 16h14" />
              <circle cx="9" cy="8" r="2" />
              <circle cx="15" cy="16" r="2" />
            </template>
            <template v-else>
              <path d="M12 3 3 7.5v9L12 21l9-4.5v-9L12 3zM3 7.5 12 12l9-4.5M12 12v9" />
            </template>
          </svg>
          <span>{{ item.label }}</span>
        </button>
      </div>

      <div class="rail-foot">
        <span v-if="environment">v{{ environment.core }}</span>
      </div>
    </nav>

    <main class="view">
      <Transition name="page" mode="out-in">
        <component :is="current" :key="page" />
      </Transition>
    </main>
  </div>
</template>

<style scoped>
.app {
  display: grid;
  grid-template-columns: 88px 1fr;
  height: 100%;
}

.rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  padding: 18px 0 14px;
  border-right: 1px solid rgba(255, 255, 255, 0.06);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.008));
}

.logo {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: #fff;
  background: linear-gradient(135deg, #5b8cff, #3ad9b0);
  box-shadow: 0 10px 24px -10px rgba(91, 140, 255, 0.9);
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  padding: 0 10px;
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  padding: 10px 0;
  border: none;
  border-radius: 12px;
  background: transparent;
  color: var(--muted);
  font-size: 11px;
  cursor: pointer;
  transition: color 160ms ease, background 160ms ease, transform 160ms ease;
}

.nav-item svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.6;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.nav-item:hover {
  color: var(--text);
  background: rgba(255, 255, 255, 0.05);
}

.nav-item.active {
  color: #fff;
  background: linear-gradient(160deg, rgba(91, 140, 255, 0.25), rgba(58, 217, 176, 0.12));
  box-shadow: inset 0 0 0 1px rgba(91, 140, 255, 0.35);
}

.rail-foot {
  margin-top: auto;
  font-size: 10px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.view {
  min-width: 0;
  min-height: 0;
  padding: 20px 24px;
  overflow: hidden;
}

.page-enter-active,
.page-leave-active {
  transition: opacity 180ms ease, transform 180ms ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

@media (prefers-reduced-motion: reduce) {
  .page-enter-active,
  .page-leave-active {
    transition: none;
  }
}
</style>
