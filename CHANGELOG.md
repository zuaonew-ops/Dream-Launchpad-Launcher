# 更新日志

本项目从 .NET 版（`main` 分支）切换为 Rust + Tauri 重写，版本号重新从 0.1.0 开始。

## [0.1.0] - 2026-10-20

### 新增

- Rust workspace：`dlp-core`（核心库）+ `dlp-desktop`（Tauri 壳）
- `dlp-core` 基础路径模块（默认游戏目录、版本独立目录），含单元测试
- Tauri 2 桌面壳：主窗口、`platform_info` 命令、capabilities 配置、NSIS/deb/AppImage 打包配置
- Vue 3 + Vite + TypeScript 前端（`ui/`），通过 `invoke` 读取运行环境信息
- 图标生成脚本（零依赖，输出 PNG + ICO）

### 变更

- 目录职责：`src/` = 核心逻辑，`src-tauri/` = 壳，`ui/` = 前端
- 归档 .NET 版设计文档到 `docs/legacy/`
- 新增 [REQ-001](docs/requirements/REQ-001-启动器骨架.md) 需求边界文档

### 移除

- .NET 方案（DLP.App / DLP.Core / DLP.Cli / tests）—— 代码保留在 `main` 分支

### 为什么

Windows 7 与 WPF 的组合无法覆盖 Linux，而目标平台包含 Linux x86_64 与 arm64。
Rust 核心可脱离界面单测，Tauri 用系统 WebView 把安装包压在个位数到十几 MB。
