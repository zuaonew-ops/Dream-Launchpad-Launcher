<script setup lang="ts">
import type { VersionSummary } from "../types";

defineProps<{
  versions: VersionSummary[];
  selected: string | null;
  loading: boolean;
}>();

const emit = defineEmits<{ select: [id: string] }>();
</script>

<template>
  <div class="list-wrap">
    <div v-if="loading" class="skeletons">
      <div v-for="i in 6" :key="i" class="skeleton" :style="{ animationDelay: `${i * 60}ms` }" />
    </div>

    <TransitionGroup v-else tag="ul" name="row" class="rows">
      <li
        v-for="(version, index) in versions"
        :key="version.id"
        class="row"
        :class="{ active: version.id === selected }"
        :style="{ '--delay': `${Math.min(index, 20) * 22}ms` }"
        @click="emit('select', version.id)"
      >
        <span class="dot" :class="{ on: version.installed }" />
        <span class="id">{{ version.id }}</span>
        <span class="tag">{{ version.installed ? "已安装" : "未安装" }}</span>
        <span class="time">{{ version.release_time.slice(0, 10) }}</span>
      </li>
    </TransitionGroup>

    <p v-if="!loading && versions.length === 0" class="empty">没有可显示的版本</p>
  </div>
</template>

<style scoped>
.list-wrap {
  min-height: 0;
  height: 100%;
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow-y: auto;
  height: 100%;
  padding-right: 6px;
}

.row {
  display: grid;
  grid-template-columns: 10px 1fr auto auto;
  align-items: center;
  gap: 12px;
  padding: 11px 14px;
  border-radius: 11px;
  border: 1px solid transparent;
  background: rgba(255, 255, 255, 0.02);
  cursor: pointer;
  transition:
    background 160ms ease,
    border-color 160ms ease,
    transform 160ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
  animation: row-in 380ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: var(--delay, 0ms);
}

.row:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.08);
  transform: translateX(3px);
}

.row.active {
  background: linear-gradient(90deg, rgba(76, 141, 255, 0.16), rgba(34, 193, 164, 0.07));
  border-color: rgba(76, 141, 255, 0.45);
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #3a4152;
  transition: background 200ms ease, box-shadow 200ms ease;
}

.dot.on {
  background: #22c1a4;
  box-shadow: 0 0 0 3px rgba(34, 193, 164, 0.16);
}

.id {
  font-size: 14px;
  font-variant-numeric: tabular-nums;
}

.tag,
.time {
  font-size: 12px;
  color: var(--muted);
}

.time {
  font-variant-numeric: tabular-nums;
}

.empty {
  color: var(--muted);
  font-size: 13px;
  padding: 24px 4px;
}

.skeletons {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.skeleton {
  height: 42px;
  border-radius: 11px;
  background: linear-gradient(90deg, #1a1e27 25%, #232936 37%, #1a1e27 63%);
  background-size: 400% 100%;
  animation: shimmer 1.4s ease infinite;
}

.row-enter-active,
.row-leave-active {
  transition: opacity 200ms ease, transform 200ms ease;
}

.row-enter-from,
.row-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

@keyframes row-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes shimmer {
  0% {
    background-position: 100% 0;
  }
  100% {
    background-position: 0 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .row,
  .skeleton {
    animation: none;
  }
}
</style>
