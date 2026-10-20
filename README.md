# Dream Launchpad Launcher

从零设计的 Minecraft: Java Edition 启动器（代号 **DLP**）。

- 核心：Rust（`dlp-core`，无 GUI 依赖，可单测）
- 桌面壳：[Tauri 2](https://tauri.app/)
- 界面：React 19 + Vite + TypeScript

## 仓库与产物的边界

这个仓库**只放源码**，构建产物和测试数据都不进来：

| 位置 | 放什么 |
| --- | --- |
| `D:\mcqdq`（本仓库） | 源码 + 文档；`dist/`、`node_modules/`、`target/` 均被忽略 |
| `D:\dlp-artifacts\release\<版本>\` | 打包产物：`DreamLaunchpad.exe` + `WebView2Loader.dll`、NSIS 安装包、便携 zip |
| `D:\dlp-artifacts\test-data\.minecraft` | 本机测试用的游戏目录（`DLP_HOME` 指向这里） |

分支语义也要分清：

- `main` —— .NET 版（历史实现，已停更）
- `feat/rust-tauri` —— Rust + Tauri 重写版，也就是现在这个产品


## 当前功能（0.8.0）

五个页面：

- **主页**：当前版本、账号、Java 概况、推荐版本；没有安装任何版本时给出明确空态
- **下载**：版本按主版本分组（26 / 20.1 / 1.20 …），支持正式版 / 快照 / 已安装 / 本地筛选；
  安装时可勾选 **Fabric / Quilt / Forge / NeoForge** 与 **OptiFine**
- **内容**：模组启用 / 禁用（`.disabled` 后缀）与删除、存档列表
- **设置**：每个版本独立的内存 / Java / 版本隔离 / 额外 JVM 参数，浅色主题
- **工具**：目录入口、日志查看、环境信息

能力：

- 扫描 `versions/` 下所有版本（含别的启动器装的），本地版本标「本地」
- 原版安装（客户端 jar / 依赖库 / natives / 资源），SHA1 校验、16–32 并发，>4MB 文件分片多连接
- 离线登录（UUID v3），自动挑选合适的 Java
- 版本隔离（默认开）与游戏默认中文（`lang:zh_cn`）
- 下载走 BMCLAPI 镜像，失败回退官方；便携模式（exe 同目录 `.minecraft`）
- 未实现：微软正版登录、Java 自动下载、整合包（CurseForge / Modrinth）

## 目标平台

| 平台 | 状态 |
| --- | --- |
| Windows 10 1809+ (x64) | 主要目标 |
| Linux x86_64 | 主要目标 |
| Linux arm64 | 主要目标（游戏本体在 arm64 上的可运行性待实测） |
| Windows 7 | **不做**（见 [REQ-001](docs/requirements/REQ-001-启动器骨架.md) §4） |
| macOS | 本期不做 |

## 数据目录

默认是**可执行文件同目录的 `.minecraft`**（绿色版，拷走整个文件夹就能带走存档）。
如果那个目录不可写（装在 Program Files 之类），自动退回 `%APPDATA%\DreamLaunchpad`。
用环境变量 `DLP_HOME` 可以指定任意目录。

下载默认走 [BMCLAPI](https://bmclapidoc.bangbang93.com/)：清单、客户端 jar、依赖库、资源对象优先镜像，
失败自动回退官方源。

## 结构

```
Cargo.toml          # Rust workspace
src/                # dlp-core：核心逻辑（清单 / 下载 / 校验 / Java / 启动 / 账号）
src-tauri/          # Tauri 壳：窗口、命令、打包配置、图标
ui/                 # React 19 前端源码：pages/（四个页面）+ components/ + store.ts（构建产物输出到根目录 dist/）
docs/               # 需求、调研、设计
scripts/            # 开发辅助脚本（图标生成等）
```

## 本地开发

```bash
npm install                 # 前端依赖
npm run tauri dev           # 起开发环境（Vite + Tauri 窗口）
npm run build               # 只构建前端
cargo test -p dlp-core      # 核心库单测
npm run tauri build         # 打包安装包（Windows: NSIS）
```

### 本机（Windows + MinGW）注意事项

这台机器没有 MSVC，Rust 走 `x86_64-pc-windows-gnu`，编译前需要：

```powershell
$env:CARGO_HOME='D:\rust\cargo'; $env:RUSTUP_HOME='D:\rust\rustup'
$env:RUSTC='D:\rust\bin2\rustc.exe'; $env:RUSTDOC='D:\rust\bin2\rustdoc.exe'
$env:PATH="D:\toolchain\mingw64\bin;D:\toolchain\nasm\nasm-2.16.03;$env:PATH"
```

- **项目路径必须是纯 ASCII**：MinGW 的 `ld.exe` / `windres.exe` 不认中文路径
- `ring`（rustls 依赖）需要 NASM
- `[lib] crate-type` 不能含 `cdylib`，否则 Windows 导出序号溢出

## 文档

- [REQ-001 启动器骨架](docs/requirements/REQ-001-启动器骨架.md)
- [C 语言 GUI 框架调研](docs/research/C-GUI框架调研.md)
- [PCL / PCL-CE 参考调研](docs/PCL-参考仓库调研.md)
- [上一版 .NET 设计（已归档）](docs/legacy/设计-DotNet版.md)
