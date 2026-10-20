<script setup lang="ts">
import type { Environment, JavaInfo } from "../types";

const username = defineModel<string>("username", { required: true });
const memory = defineModel<number>("memory", { required: true });
const isolate = defineModel<boolean>("isolate", { required: true });
const javaPath = defineModel<string>("javaPath", { required: true });

defineProps<{
  javas: JavaInfo[];
  environment: Environment | null;
}>();
defineEmits<{ openDir: [] }>();
</script>

<template>
  <aside class="side">
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
      <label class="label" for="java">Java 运行时</label>
      <select id="java" v-model="javaPath" class="input select">
        <option value="">自动选择（按版本要求）</option>
        <option v-for="j in javas" :key="j.path" :value="j.path">
          Java {{ j.major }} · {{ j.version }}
        </option>
      </select>
      <p class="hint" v-if="javas.length === 0">没有探测到 Java，请先安装 17 或更高版本</p>
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
      <button class="ghost" @click="$emit('openDir')">打开文件夹</button>
    </section>
  </aside>
</template>

<style scoped>
.side {
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 16px;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
  animation: rise 520ms cubic-bezier(0.16, 1, 0.3, 1) both;
  animation-delay: 60ms;
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

.select {
  font-size: 13px;
  appearance: none;
  cursor: pointer;
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

.meta code {
  font-size: 11px;
  color: var(--muted);
  word-break: break-all;
  line-height: 1.6;
}

.ghost {
  align-self: flex-start;
  background: transparent;
  border: 1px solid var(--line);
  color: var(--muted);
  border-radius: 9px;
  padding: 5px 11px;
  font-size: 12px;
  cursor: pointer;
  transition: color 160ms ease, border-color 160ms ease, transform 160ms ease;
}

.ghost:hover {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.25);
  transform: translateY(-1px);
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
  .side {
    animation: none;
  }
}
</style>
