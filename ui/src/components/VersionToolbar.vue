<script setup lang="ts">
import type { TypeFilter } from "../composables/useLauncher";

const keyword = defineModel<string>("keyword", { required: true });
const filter = defineModel<TypeFilter>("filter", { required: true });

defineProps<{
  loading: boolean;
  total: number;
}>();
defineEmits<{ refresh: [] }>();

const tabs: { key: TypeFilter; label: string }[] = [
  { key: "release", label: "正式版" },
  { key: "snapshot", label: "快照" },
  { key: "installed", label: "已安装" },
  { key: "all", label: "全部" },
];
</script>

<template>
  <div class="toolbar">
    <div class="tabs">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab"
        :class="{ active: filter === tab.key }"
        @click="filter = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <div class="search">
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
        <path
          d="M10 4a6 6 0 1 0 3.8 10.6l4.3 4.3 1.4-1.4-4.3-4.3A6 6 0 0 0 10 4zm0 2a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"
          fill="currentColor"
        />
      </svg>
      <input v-model="keyword" placeholder="搜索版本号" spellcheck="false" />
      <span class="count">{{ total }}</span>
    </div>

    <button class="ghost" :disabled="loading" @click="$emit('refresh')">
      {{ loading ? "刷新中" : "刷新" }}
    </button>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
}

.tabs {
  display: flex;
  gap: 2px;
  padding: 3px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid var(--line);
}

.tab {
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  padding: 5px 11px;
  border-radius: 8px;
  cursor: pointer;
  transition: color 150ms ease, background 150ms ease;
}

.tab:hover {
  color: var(--text);
}

.tab.active {
  color: #fff;
  background: linear-gradient(135deg, rgba(91, 140, 255, 0.85), rgba(58, 217, 176, 0.6));
  box-shadow: 0 4px 14px -6px rgba(91, 140, 255, 0.9);
}

.search {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 7px 11px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(0, 0, 0, 0.2);
  color: var(--muted);
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.search:focus-within {
  border-color: rgba(91, 140, 255, 0.55);
  box-shadow: 0 0 0 3px rgba(91, 140, 255, 0.1);
}

.search input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
}

.count {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.ghost {
  background: transparent;
  border: 1px solid var(--line);
  color: var(--muted);
  border-radius: 10px;
  padding: 7px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: color 150ms ease, border-color 150ms ease, transform 150ms ease;
}

.ghost:hover:not(:disabled) {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.22);
  transform: translateY(-1px);
}
</style>
