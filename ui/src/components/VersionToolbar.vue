<script setup lang="ts">
const keyword = defineModel<string>("keyword", { required: true });
const snapshots = defineModel<boolean>("snapshots", { required: true });

defineProps<{
  loading: boolean;
  total: number;
}>();
defineEmits<{ refresh: [] }>();
</script>

<template>
  <div class="toolbar">
    <div class="search">
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
        <path
          d="M10 4a6 6 0 1 0 3.8 10.6l4.3 4.3 1.4-1.4-4.3-4.3A6 6 0 0 0 10 4zm0 2a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"
          fill="currentColor"
        />
      </svg>
      <input v-model="keyword" placeholder="搜索版本" spellcheck="false" />
    </div>

    <label class="toggle">
      <input v-model="snapshots" type="checkbox" />
      <span>快照</span>
    </label>

    <span class="count">{{ total }}</span>

    <button class="ghost" :disabled="loading" @click="$emit('refresh')">
      {{ loading ? "刷新中" : "刷新" }}
    </button>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.search {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 1;
  padding: 7px 11px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(0, 0, 0, 0.22);
  color: var(--muted);
  transition: border-color 160ms ease, box-shadow 160ms ease;
}

.search:focus-within {
  border-color: rgba(76, 141, 255, 0.6);
  box-shadow: 0 0 0 3px rgba(76, 141, 255, 0.12);
}

.search input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
}

.toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--muted);
  cursor: pointer;
  user-select: none;
}

.toggle input {
  accent-color: #4c8dff;
}

.count {
  font-size: 12px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
  min-width: 32px;
  text-align: right;
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
</style>
