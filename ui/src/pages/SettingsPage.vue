<script setup lang="ts">
import { useLauncher } from "../composables/useLauncher";

const { username, memory, isolate, javaPath, javas, environment, activeJava, openDir } = useLauncher();
</script>

<template>
  <div class="page">
    <header class="page-head">
      <h1>设置</h1>
      <p class="desc">修改后立即生效，保存在本地</p>
    </header>

    <div class="grid">
      <section class="panel">
        <h2>账号</h2>
        <label class="field">
          <span class="label">离线昵称</span>
          <input v-model="username" class="input" maxlength="16" spellcheck="false" />
        </label>
        <p class="hint">离线登录只在本地生成 UUID（UUID v3），不联网校验</p>
      </section>

      <section class="panel">
        <h2>性能</h2>
        <label class="field">
          <span class="label">内存上限</span>
          <div class="row">
            <input v-model.number="memory" type="range" min="1024" max="8192" step="512" />
            <span class="value">{{ memory }} MB</span>
          </div>
        </label>

        <label class="field">
          <span class="label">Java 运行时</span>
          <select v-model="javaPath" class="input">
            <option value="">自动选择（按版本要求）</option>
            <option v-for="j in javas" :key="j.path" :value="j.path">
              Java {{ j.major }} · {{ j.version }}
            </option>
          </select>
        </label>
        <p class="hint">
          {{ activeJava ? `已固定使用 ${activeJava.path}` : "自动模式会按版本要求挑最低可用版本" }}
        </p>
      </section>

      <section class="panel">
        <h2>存储</h2>
        <label class="switch-row">
          <div>
            <span class="label">版本隔离</span>
            <p class="hint">每个版本独立存档与 mods（versions/&lt;版本&gt;/）</p>
          </div>
          <input v-model="isolate" class="switch" type="checkbox" />
        </label>

        <div class="field">
          <span class="label">数据目录</span>
          <code class="path">{{ environment?.data_dir ?? "…" }}</code>
          <div class="row">
            <button class="ghost" @click="openDir('data')">打开目录</button>
            <button class="ghost" @click="openDir('logs')">打开日志</button>
          </div>
        </div>
      </section>
    </div>
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

.page-head h1 {
  font-size: 22px;
  font-weight: 650;
}

.desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--muted);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
  align-items: start;
}

.panel {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.012));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

h2 {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.label {
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.input {
  width: 100%;
  padding: 9px 12px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(0, 0, 0, 0.22);
  color: var(--text);
  font-size: 13px;
  outline: none;
  transition: border-color 150ms ease, box-shadow 150ms ease;
}

.input:focus {
  border-color: rgba(91, 140, 255, 0.6);
  box-shadow: 0 0 0 3px rgba(91, 140, 255, 0.12);
}

.row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.row input[type="range"] {
  flex: 1;
  accent-color: #5b8cff;
}

.value {
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  min-width: 66px;
  text-align: right;
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  cursor: pointer;
}

.switch {
  appearance: none;
  width: 38px;
  height: 21px;
  border-radius: 999px;
  background: #232a37;
  position: relative;
  cursor: pointer;
  transition: background 180ms ease;
  flex: none;
}

.switch::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background: #cfd6e4;
  transition: transform 180ms cubic-bezier(0.2, 0.9, 0.3, 1.4);
}

.switch:checked {
  background: linear-gradient(135deg, #5b8cff, #3ad9b0);
}

.switch:checked::after {
  transform: translateX(17px);
  background: #fff;
}

.path {
  font-size: 11px;
  color: var(--muted);
  word-break: break-all;
  line-height: 1.6;
}

.hint {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.65;
}

.ghost {
  background: transparent;
  border: 1px solid var(--line);
  color: var(--muted);
  border-radius: 9px;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: color 150ms ease, border-color 150ms ease, transform 150ms ease;
}

.ghost:hover {
  color: var(--text);
  border-color: rgba(255, 255, 255, 0.22);
  transform: translateY(-1px);
}
</style>
