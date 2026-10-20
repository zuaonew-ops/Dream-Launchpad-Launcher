<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import VersionList from "./components/VersionList.vue";
import ProgressPanel from "./components/ProgressPanel.vue";
import { useLauncher } from "./composables/useLauncher";

const {
  versions,
  environment,
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
        {{ environment.os }} · {{ environment.arch }} · core {{ environment.core }}
      </div>
    </header>

    <main class="body">
      <aside class="panel side">
        <section class="block">
          <label class="label" for="nick">离线昵称</label>
          <input
            id="nick"
            v-model="username"
            class="input"
            maxlength="16"
            spellcheck="false"
            placeholder="Steve"
          />
          <p class="hint">离线登录只在本地生成 UUID，不需要账号密码</p>
        </section>

        <section class="block">
          <label class="label" for="mem">内存上限</label>
          <div class="mem">
            <input id="mem" v-model.number="memory" type="range" min="1024" max="8192" step="512" />
            <span class="mem-val">{{ memory }} MB</span>
          </div>
        </section>

        <section class="block">
          <label class="switch">
            <input v-model="isolate" type="checkbox" />
            <span>版本隔离</span>
          </label>
          <p class="hint">开启后每个版本独立存档与 mods（versions/&lt;版本&gt;/）</p>
        </section>

        <section class="block meta">
          <span class="label">数据目录</span>
          <code>{{ environment?.data_dir ?? "…" }}</code>
        </section>
      </aside>

      <section class="panel main">
        <div class="list-head">
          <span class="label">正式版本</span>
          <button class="ghost" :disabled="loading" @click="refresh">
            {{ loading ? "刷新中" : "刷新" }}
          </button>
        </div>
        <VersionList
          :versions="versions"
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
        </div>

        <ProgressPanel
          v-if="install"
          :stage="install.stage"
          :percent="install.percent"
        />

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
  padding: 22px 26px 18px;
  gap: 16px;
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
  width: 46px;
  height: 46px;
  border-radius: 14px;
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
  grid-template-columns: 300px 1fr;
  gap: 14px;
  min-height: 0;
}

.panel {
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 16px;
  min-height: 0;
}

.side {
  display: flex;
  flex-direction: column;
  gap: 18px;
  animation: rise 520ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 60ms;
}

.main {
  display: flex;
  flex-direction: column;
  gap: 10px;
  animation: rise 560ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 110ms;
}

.block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.label {
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font-size: 14px;
  outline: none;
  transition: border-color 160ms ease, box-shadow 160ms ease;
}

.input:focus {
  border-color: rgba(76, 141, 255, 0.7);
  box-shadow: 0 0 0 3px rgba(76, 141, 255, 0.15);
}

.hint {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.6;
}

.mem {
  display: flex;
  align-items: center;
  gap: 12px;
}

input[type="range"] {
  flex: 1;
  accent-color: #4c8dff;
}

.mem-val {
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  color: var(--text);
  min-width: 66px;
  text-align: right;
}

.meta code {
  font-size: 11px;
  color: var(--muted);
  word-break: break-all;
  line-height: 1.6;
}

.list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.ghost {
  background: transparent;
  border: 1px solid var(--line);
  color: var(--muted);
  border-radius: 9px;
  padding: 5px 11px;
  font-size: 12px;
  cursor: pointer;
  transition: color 160ms ease, border-color 160ms ease, transform 160ms ease;
}

.ghost:hover:not(:disabled) {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-1px);
}

.bottom {
  display: flex;
  flex-direction: column;
  gap: 10px;
  animation: rise 600ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 160ms;
}

.action {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 18px;
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 12px 16px;
}

.picked {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 92px;
}

.picked strong {
  font-size: 15px;
  font-variant-numeric: tabular-nums;
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

.switch {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 13px;
  cursor: pointer;
}

.switch input {
  appearance: none;
  width: 36px;
  height: 20px;
  border-radius: 999px;
  background: #2a303c;
  position: relative;
  cursor: pointer;
  transition: background 180ms ease;
}

.switch input::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #cfd6e4;
  transition: transform 180ms cubic-bezier(0.2, 0.9, 0.3, 1.4);
}

.switch input:checked {
  background: linear-gradient(135deg, #4c8dff, #22c1a4);
}

.switch input:checked::after {
  transform: translateX(16px);
  background: #fff;
}

@media (prefers-reduced-motion: reduce) {
  .top,
  .side,
  .main,
  .bottom {
    animation: none;
  }
}
</style>
