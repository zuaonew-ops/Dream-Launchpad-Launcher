<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface PlatformInfo {
  os: string;
  arch: string;
  tauri: string;
  launcher: string;
  core: string;
  game_dir: string;
}

const info = ref<PlatformInfo | null>(null);
const error = ref("");

onMounted(async () => {
  try {
    info.value = await invoke<PlatformInfo>("platform_info");
  } catch (e) {
    error.value = String(e);
  }
});
</script>

<template>
  <main class="shell">
    <header class="hero">
      <div class="mark">DLP</div>
      <div>
        <h1>Dream Launchpad Launcher</h1>
        <p class="sub">跨平台 Minecraft 启动器 · 桌面端（Rust + Tauri 2）</p>
      </div>
    </header>

    <section class="card">
      <h2>运行环境</h2>
      <dl v-if="info" class="grid">
        <div><dt>系统</dt><dd>{{ info.os }} / {{ info.arch }}</dd></div>
        <div><dt>Tauri</dt><dd>{{ info.tauri }}</dd></div>
        <div><dt>启动器</dt><dd>{{ info.launcher }}</dd></div>
        <div><dt>核心库</dt><dd>dlp-core {{ info.core }}</dd></div>
        <div class="wide"><dt>游戏目录</dt><dd>{{ info.game_dir }}</dd></div>
      </dl>
      <p v-else-if="error" class="err">Rust 侧命令调用失败：{{ error }}</p>
      <p v-else class="muted">正在读取…</p>
    </section>

    <section class="card muted-card">
      <h2>下一步</h2>
      <ul>
        <li>版本清单 / 文件下载 / SHA1 校验（Rust 侧）</li>
        <li>微软正版登录 + 离线登录</li>
        <li>Java 运行时探测与管理</li>
        <li>Mod 加载器安装（Fabric / Forge / NeoForge）</li>
      </ul>
    </section>
  </main>
</template>

<style scoped>
.shell {
  height: 100%;
  padding: 32px 36px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  overflow-y: auto;
}

.hero {
  display: flex;
  align-items: center;
  gap: 16px;
}

.mark {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  display: grid;
  place-items: center;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: #fff;
  background: linear-gradient(135deg, #4c8dff, #22c1a4);
  box-shadow: 0 12px 30px -12px #4c8dff88;
}

h1 {
  font-size: 20px;
  font-weight: 650;
}

.sub {
  margin-top: 4px;
  font-size: 13px;
  color: var(--muted);
}

.card {
  background: var(--bg-soft);
  border: 1px solid var(--line);
  border-radius: 14px;
  padding: 18px 20px;
}

h2 {
  font-size: 13px;
  font-weight: 600;
  color: var(--muted);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  margin-bottom: 12px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
}

.wide {
  grid-column: 1 / -1;
  word-break: break-all;
}

dt {
  font-size: 12px;
  color: var(--muted);
}

dd {
  margin-top: 2px;
  font-size: 14px;
  font-variant-numeric: tabular-nums;
}

.muted,
.muted-card {
  color: var(--muted);
}

ul {
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.9;
}

.err {
  color: #ff7a7a;
  font-size: 13px;
}
</style>
