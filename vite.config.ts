import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// 前端源码在 ui/，构建产物落在仓库根的 dist/（供 Tauri 打包用）
export default defineConfig({
  root: "ui",
  plugins: [react()],

  // Tauri 需要一个固定端口，端口被占用时直接失败而不是自动换端口
  clearScreen: false,
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
});
