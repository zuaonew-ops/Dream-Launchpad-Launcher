<script setup lang="ts">
import ProgressPanel from "../components/ProgressPanel.vue";
import VersionList from "../components/VersionList.vue";
import VersionToolbar from "../components/VersionToolbar.vue";
import { useLauncher } from "../composables/useLauncher";

const {
  groups,
  filtered,
  selected,
  selectedVersion,
  search,
  typeFilter,
  loading,
  busy,
  install,
  error,
  notice,
  refresh,
  installSelected,
  launchSelected,
} = useLauncher();
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1>下载</h1>
        <p class="desc">选择版本安装，原版文件经 BMCLAPI 镜像下载</p>
      </div>
      <span class="pill">{{ filtered.length }} 个版本</span>
    </header>

    <section class="panel list-panel">
      <VersionToolbar
        v-model:keyword="search"
        v-model:filter="typeFilter"
        :loading="loading"
        :total="filtered.length"
        @refresh="refresh"
      />
      <VersionList
        :groups="groups"
        :selected="selected"
        :loading="loading"
        @select="selected = $event"
      />
    </section>

    <footer class="action">
      <div class="picked">
        <span class="label">当前选择</span>
        <strong>{{ selectedVersion?.id ?? "未选择" }}</strong>
        <span class="badge" :class="{ ok: selectedVersion?.installed }">
          {{ selectedVersion ? (selectedVersion.installed ? "已安装" : "未安装") : "—" }}
        </span>
      </div>

      <div class="progress-slot">
        <ProgressPanel
          v-if="install"
          :stage="install.stage"
          :percent="install.percent"
          :done="install.done"
          :total="install.total"
        />
        <p v-else-if="error" class="msg error">{{ error }}</p>
        <p v-else-if="notice" class="msg ok">{{ notice }}</p>
      </div>

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
    </footer>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: 100%;
  min-height: 0;
}

.page-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  padding: 0 2px;
}

h1 {
  font-size: 22px;
  font-weight: 650;
}

.desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--muted);
}

.pill {
  font-size: 11px;
  color: var(--muted);
  padding: 3px 10px;
  border-radius: 999px;
  border: 1px solid var(--line);
}

.panel {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.012));
  border: 1px solid var(--line);
  border-radius: var(--radius);
}

.list-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  overflow: hidden;
}

.action {
  display: grid;
  grid-template-columns: minmax(160px, auto) 1fr auto;
  align-items: center;
  gap: 18px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.015));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 12px 16px;
}

.picked {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}

.picked strong {
  font-size: 16px;
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
  color: #3ad9b0;
  background: rgba(58, 217, 176, 0.12);
}

.progress-slot {
  min-height: 34px;
  display: flex;
  align-items: center;
}

.progress-slot > * {
  width: 100%;
}

.msg {
  font-size: 12px;
}

.msg.error {
  color: #ff8080;
}

.msg.ok {
  color: #3ad9b0;
}
</style>
