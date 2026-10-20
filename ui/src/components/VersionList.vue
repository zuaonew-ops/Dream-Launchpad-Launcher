<script setup lang="ts">
import type { VersionSummary } from "../types";

defineProps<{
  groups: { key: string; items: VersionSummary[] }[];
  selected: string | null;
  loading: boolean;
}>();

const emit = defineEmits<{ select: [id: string] }>();
</script>

<template>
  <div class="list-wrap">
    <div v-if="loading" class="skeletons">
      <div v-for="i in 7" :key="i" class="skeleton" :style="{ animationDelay: `${i * 60}ms` }" />
    </div>

    <div v-else class="groups">
      <section v-for="group in groups" :key="group.key" class="group">
        <header class="group-head">
          <span class="group-name">{{ group.key }}</span>
          <span class="group-count">{{ group.items.length }}</span>
        </header>

        <ul class="rows">
          <li
            v-for="version in group.items"
            :key="version.id"
            class="row"
            :class="{ active: version.id === selected }"
            @click="emit('select', version.id)"
          >
            <span class="bar" />
            <span class="id">{{ version.id }}</span>
            <span class="kind" :class="version.kind">{{ version.kind === "release" ? "正式" : "快照" }}</span>
            <span class="state" :class="{ on: version.installed }">
              {{ version.installed ? "已安装" : "未安装" }}
            </span>
            <span class="time">{{ version.release_time.slice(0, 10) }}</span>
          </li>
        </ul>
      </section>

      <p v-if="groups.length === 0" class="empty">没有匹配的版本</p>
    </div>
  </div>
</template>

<style scoped>
.list-wrap {
  min-height: 0;
  height: 100%;
  overflow-y: auto;
  padding-right: 4px;
}

.group {
  margin-bottom: 6px;
}

.group-head {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 6px 6px;
  font-size: 11px;
  letter-spacing: 0.12em;
  color: var(--muted);
  text-transform: uppercase;
  background: linear-gradient(180deg, var(--bg-soft) 70%, transparent);
  backdrop-filter: blur(6px);
}

.group-name {
  font-weight: 600;
}

.group-count {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.06);
  letter-spacing: 0;
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.row {
  position: relative;
  display: grid;
  grid-template-columns: 1fr auto auto auto;
  align-items: center;
  gap: 12px;
  padding: 10px 14px 10px 18px;
  border-radius: 10px;
  border: 1px solid transparent;
  cursor: pointer;
  transition:
    background 150ms ease,
    border-color 150ms ease,
    transform 150ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
}

.row:hover {
  background: rgba(255, 255, 255, 0.045);
  border-color: rgba(255, 255, 255, 0.08);
  transform: translateX(3px);
}

.row.active {
  background: linear-gradient(90deg, rgba(91, 140, 255, 0.18), rgba(58, 217, 176, 0.06));
  border-color: rgba(91, 140, 255, 0.42);
}

.bar {
  position: absolute;
  left: 6px;
  top: 50%;
  width: 3px;
  height: 0;
  border-radius: 999px;
  background: linear-gradient(180deg, #5b8cff, #3ad9b0);
  transform: translateY(-50%);
  transition: height 180ms cubic-bezier(0.2, 0.9, 0.3, 1.2);
}

.row.active .bar {
  height: 18px;
}

.id {
  font-size: 14px;
  font-variant-numeric: tabular-nums;
}

.kind {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 6px;
  color: #8fa4c8;
  background: rgba(91, 140, 255, 0.1);
}

.kind.snapshot {
  color: #c8a4ff;
  background: rgba(168, 120, 255, 0.12);
}

.state {
  font-size: 12px;
  color: var(--muted);
}

.state.on {
  color: #3ad9b0;
}

.time {
  font-size: 12px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}

.empty {
  color: var(--muted);
  font-size: 13px;
  padding: 30px 6px;
  text-align: center;
}

.skeletons {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.skeleton {
  height: 40px;
  border-radius: 10px;
  background: linear-gradient(90deg, #161a23 25%, #1e2430 37%, #161a23 63%);
  background-size: 400% 100%;
  animation: shimmer 1.4s ease infinite;
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
  .skeleton,
  .row {
    animation: none;
  }
}
</style>
