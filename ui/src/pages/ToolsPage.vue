<script setup lang="ts">
import { useLauncher } from "../composables/useLauncher";

const { environment, javas, installedCount, versions, openDir } = useLauncher();
</script>

<template>
  <div class="page">
    <header class="page-head">
      <h1>工具</h1>
      <p class="desc">目录与运行环境</p>
    </header>

    <section class="panel">
      <h2>目录</h2>
      <div class="buttons">
        <button class="tool" @click="openDir('data')">
          <span class="tool-title">游戏目录</span>
          <span class="tool-hint">存档 / 配置 / mods</span>
        </button>
        <button class="tool" @click="openDir('logs')">
          <span class="tool-title">日志</span>
          <span class="tool-hint">启动器与游戏输出</span>
        </button>
        <button class="tool" @click="openDir('versions')">
          <span class="tool-title">版本目录</span>
          <span class="tool-hint">每个版本一个文件夹</span>
        </button>
      </div>
    </section>

    <section class="panel">
      <h2>环境</h2>
      <dl class="info">
        <div><dt>系统</dt><dd>{{ environment?.os }} / {{ environment?.arch }}</dd></div>
        <div><dt>Tauri</dt><dd>{{ environment?.tauri }}</dd></div>
        <div><dt>核心库</dt><dd>dlp-core {{ environment?.core }}</dd></div>
        <div><dt>版本</dt><dd>{{ installedCount }} 已安装 / {{ versions.length }} 可用</dd></div>
        <div><dt>Java</dt><dd>{{ javas.length }} 个运行时</dd></div>
        <div class="wide"><dt>数据目录</dt><dd>{{ environment?.data_dir }}</dd></div>
      </dl>
    </section>

    <p class="todo">更多工具（皮肤、日志分析、存档备份）开发中</p>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 14px;
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

.panel {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.012));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 18px;
}

h2 {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}

.buttons {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 10px;
}

.tool {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
  padding: 14px 16px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.03);
  color: var(--text);
  cursor: pointer;
  transition: transform 150ms ease, border-color 150ms ease, background 150ms ease;
}

.tool:hover {
  transform: translateY(-2px);
  border-color: rgba(91, 140, 255, 0.45);
  background: rgba(91, 140, 255, 0.08);
}

.tool-title {
  font-size: 14px;
  font-weight: 550;
}

.tool-hint {
  font-size: 11px;
  color: var(--muted);
}

.info {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

dt {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

dd {
  margin-top: 3px;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  word-break: break-all;
}

.todo {
  margin-top: auto;
  font-size: 11px;
  color: var(--muted);
  padding: 4px 2px 0;
}
</style>
