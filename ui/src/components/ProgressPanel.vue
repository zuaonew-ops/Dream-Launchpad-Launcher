<script setup lang="ts">
defineProps<{
  stage: string;
  percent: number;
}>();
</script>

<template>
  <Transition name="panel">
    <div class="progress">
      <div class="head">
        <span class="stage">{{ stage }}</span>
        <span class="pct">{{ Math.round(percent * 100) }}%</span>
      </div>
      <div class="track">
        <div class="fill" :style="{ width: `${Math.max(3, percent * 100)}%` }" />
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.progress {
  display: flex;
  flex-direction: column;
  gap: 8px;
  animation: panel-in 260ms cubic-bezier(0.16, 1, 0.3, 1) both;
}

.head {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--muted);
}

.pct {
  font-variant-numeric: tabular-nums;
  color: var(--text);
}

.track {
  height: 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.07);
  overflow: hidden;
}

.fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #4c8dff, #22c1a4);
  background-size: 200% 100%;
  transition: width 260ms cubic-bezier(0.22, 1, 0.36, 1);
  animation: flow 2.2s linear infinite;
  box-shadow: 0 0 12px rgba(76, 141, 255, 0.45);
}

@keyframes flow {
  0% {
    background-position: 0 0;
  }
  100% {
    background-position: 200% 0;
  }
}

@keyframes panel-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.panel-enter-active,
.panel-leave-active {
  transition: opacity 200ms ease, transform 200ms ease;
}

.panel-enter-from,
.panel-leave-to {
  opacity: 0;
  transform: translateY(6px);
}

@media (prefers-reduced-motion: reduce) {
  .fill {
    animation: none;
  }
}
</style>
