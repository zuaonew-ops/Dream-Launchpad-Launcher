# 更新日志

本项目从 .NET 版（`main` 分支）切换为 Rust + Tauri 重写，版本号重新从 0.1.0 开始。

## [0.4.0] - 2026-10-20

### 新增

- **便携模式**：首次启动在可执行文件同目录创建 `.minecraft`，所有数据跟着程序走；
  目录建不出来（例如装在 Program Files 且无写权限）时自动退回用户数据目录
- **BMCLAPI 镜像**：清单、客户端 jar、依赖库、资源对象优先走镜像，失败自动回退官方源

### 变更

- 依赖库 / 客户端 / 资源的下载地址改为"镜像优先 + 官方兜底"，重试第 3 次强制换官方
- 资源索引、日志配置这类几 KB 的元数据仍走官方（镜像没有对应端点）

## [0.3.0] - 2026-10-20

### 新增

- **版本隔离**（默认开启）：每个版本以 `versions/<id>/` 作为游戏目录，存档、配置、mods 互不影响；界面可关闭
- **游戏默认中文**：首次启动写入 `options.txt` 的 `lang:zh_cn`（文件已存在则不覆盖玩家设置）
- 界面新增「版本隔离」开关，选择记在 localStorage

### 变更

- 游戏日志按版本分文件：`logs/<版本>.log`
- `launch_game` 命令新增 `isolate` 参数；启动逻辑改用 `LaunchOptions` 结构

## [0.2.0] - 2026-10-20

### 新增

- 版本清单拉取与版本列表（默认只显示正式版）
- 原版安装：客户端 jar、依赖库（rules 求值）、natives 解压、资源文件；SHA1 校验、16/32 并发、失败重试 3 次
- 版本 JSON 继承链合并（`inheritsFrom`）
- 离线账号（UUID v3）与启动参数构建（兼容新旧两种 arguments 格式）
- Java 自动探测（JAVA_HOME / PATH / 常见安装目录）并按版本要求挑选
- 桌面界面：版本列表、离线昵称、内存上限、安装进度事件、安装/启动按钮
- 开发探针：`examples/install_probe.rs`、`examples/launch_probe.rs`

### 修复

- 1.19+ 的 natives 是独立库条目，且 rules 只按操作系统筛选，需按架构二次过滤，否则会解出错误的 dll
- zip 的目录条目被当成文件写入，导致 `os error 123`

### 变更

- 核心库按职责拆分：`minecraft/`（清单 / 版本 / 规则 / 平台）、`net/`（下载）、`install/`（编排 / 资源 / natives）、`launch/`（参数 / 进程）

### 已知问题

- 1.6 及更早版本的 legacy 资源（virtual/legacy）未处理
- 未实现：微软正版登录、Java 自动下载、Fabric/Forge/NeoForge

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
