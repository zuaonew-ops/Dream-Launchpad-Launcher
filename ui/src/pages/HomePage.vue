<script setup lang="ts">
import { computed } from "vue";
import { useLauncher } from "../composables/useLauncher";

const {
  selectedVersion,
  selected,
  installedCount,
  versions,
  javas,
  latestReleases,
  username,
  notice,
  error,
  busy,
  launchPid,
  install,
  launchSelected,
  installSelected,
  openDir,
} = useLauncher();

const javaLabel = computed(() => {
  if (!javas.value.length) return "未检测到";
  const max = javas.value[0].major;
  const min = javas.value[javas.value.length - 1].major;
  return min === max ? `Java ${max}` : `Java ${min}–${max}`;
});

function primaryAction() {
  if (!selectedVersion.value) return;
  if (selectedVersion.value.installed) launchSelected();
  else installSelected();
}
</script>

<template>
  <div class="page">
    <header class="hero">
      <div>
        <p class="eyebrow">DREAM LAUNCHPAD</p>
        <h1>准备起飞</h1>
        <p class="desc">
          离线登录 · 已安装 {{ installedCount }} 个版本 · 共 {{ versions.length }} 个可用
        </p>
      </div>
      <div class="hero-actions">
        <button class="ghost" @click="openDir('data')">打开游戏目录</button>
        <button class="primary" :disabled="busy || !selectedVersion" @click="primaryAction">
          <span v-if="install">{{ Math.round(install.percent * 100) }}%</span>
          <span v-else>{{ selectedVersion?.installed ? "启动游戏" : "安装并准备" }}</span>
        </button>
      </div>
    </header>

    <div class="cards">
      <article class="card">
        <p class="card-label">当前版本</p>
        <p class="card-value">{{ selectedVersion?.id ?? "未选择" }}</p>
        <p class="card-hint">
          {{ selectedVersion?.installed ? "已安装，可直接启动" : "尚未安装" }}
          <template v-if="launchPid"> · 运行中 {{ launchPid }}</template>
        </p>
      </article>

      <article class="card">
        <p class="card-label">账号</p>
        <p class="card-value">{{ username }}</p>
        <p class="card-hint">离线模式 · UUID 本地生成</p>
      </article>

      <article class="card">
        <p class="card-label">Java 运行时</p>
        <p class="card-value">{{ javaLabel }}</p>
        <p class="card-hint">{{ javas.length }} 个可用</p>
      </article>
    </div>

    <section class="section">
      <div class="section-head">
        <h2>推荐版本</h2>
        <span class="hint">最新正式版</span>
      </div>
      <div class="quick">
        <button
          v-for="version in latestReleases"
          :key="version.id"
          class="quick-item"
          :class="{ active: selected === version.id }"
          @click="selected = version.id"
        >
          <span class="quick-id">{{ version.id }}</span>
          <span class="quick-state" :class="{ on: version.installed }">
            {{ version.installed ? "已安装" : "未安装" }}
          </span>
        </button>
      </div>
    </section>

    <section class="panel note">
      <p class="card-label">状态</p>
      <p class="card-hint">{{ notice || error || "等待操作" }}</p>
    </section>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
  overflow-y: auto;
}

.hero {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding: 6px 2px 0;
}

.eyebrow {
  font-size: 11px;
  letter-spacing: 0.22em;
  color: #6f9bff;
  margin-bottom: 8px;
}

h1 {
  font-size: 30px;
  font-weight: 650;
  letter-spacing: -0.01em;
}

.desc {
  margin-top: 6px;
  font-size: 13px;
  color: var(--muted);
}

.hero-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.card,
.panel {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.015));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 16px 18px;
}

.card-label {
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.card-value {
  margin-top: 10px;
  font-size: 22px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.card-hint {
  margin-top: 6px;
  font-size: 12px;
  color: var(--muted);
  line-height: 1.6;
}

.section-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 10px;
}

h2 {
  font-size: 13px;
  font-weight: 600;
}

.hint {
  font-size: 11px;
  color: var(--muted);
}

.quick {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
}

.quick-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  align-items: flex-start;
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.025);
  color: var(--text);
  cursor: pointer;
  transition: transform 150ms ease, border-color 150ms ease, background 150ms ease;
}

.quick-item:hover {
  transform: translateY(-2px);
  border-color: rgba(91, 140, 255, 0.42);
  background: rgba(91, 140, 255, 0.07);
}

.quick-item.active {
  border-color: rgba(91, 140, 255, 0.65);
  background: linear-gradient(160deg, rgba(91, 140, 255, 0.16), rgba(58, 217, 176, 0.06));
}

.quick-id {
  font-size: 14px;
  font-variant-numeric: tabular-nums;
}

.quick-state {
  font-size: 11px;
  color: var(--muted);
}

.quick-state.on {
  color: #3ad9b0;
}

.note {
  margin-top: auto;
}
</style>
