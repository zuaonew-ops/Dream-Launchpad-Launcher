# PCL / PCL-CE 仓库调研（2026-10-20）

## 一、Meloong-Git/PCL（官方版）

| 项 | 值 |
|---|---|
| 地址 | https://github.com/Meloong-Git/PCL |
| 描述 | Minecraft 启动器 Plain Craft Launcher（PCL） |
| Star / Fork | 7,725 / 728 |
| 贡献者 | 60 人（API Link 头 last=60） |
| 语言 | Visual Basic .NET 2,222,027 字节；C# 48,185 字节 |
| 技术栈 | WPF + VB.NET，`TargetFrameworkVersion v4.8`（.NET Framework 4.8） |
| 许可证 | 自定义《PCL 分发有限许可》+《PCL 存储库合理使用指南》（非 OSI 许可） |
| 子模块 | `MeloongCore` → github.com/Meloong-Git/MeloongCore（C#，Apache-2.0，12 星） |
| CI | `.github/workflows/build.yml` |
| 最新版本 | 2.99.0.0（2026-10-20 发布），上次 2.14.0.0（2026-10-16） |
| 结构 | `Plain Craft Launcher 2/`（FormMain、Pages、Modules{Base,Minecraft,Resource,ThirdParty}、Controls）、`PCLCS/`（C# 类库，net48，含 Launch/Minecraft/Resource/Java.cs）、`patches/`、`最新正式版.zip` |
| 分支 | `main`、`copilot/fix-7009` |

关键事实：
- 2.99.0.0 发布说明原文：**「为遵守相关法律法规，PCL 不再提供 MC 下载功能」**，即主线已砍掉 Minecraft 下载。
- 仓库代码「不是实时更新，每次发布同步一次」。
- 许可证约束（重度使用必须满足）：名称必须以 Plain Craft Launcher (PCL) 开头并加第三方后缀；关于页首位署名龙腾猫跃 + 赞助链接；公开源代码；需实现与 `McLaunchPrecheck` 一致的购买劝导；Windows 上界面主色相必须为蓝色。

## 二、PCL-Community/PCL-CE（社区版）

| 项 | 值 |
|---|---|
| 地址 | https://github.com/PCL-Community/PCL-CE |
| 描述 | PCL 社区版，由社区开发者维护 |
| Star / Fork | 4,475 / 208 |
| 贡献者 | 91 人 |
| 语言 | C# 5,457,136 字节；HLSL / HTML / Batch 少量 |
| 技术栈 | WPF + C#，`net10.0-windows`，LangVersion 14，Nullable enable，PublishSingleFile，x64/ARM64 |
| 许可证 | `Plain Craft Launcher 2/` 为自定义《PCL 分发有限许可》；**其余目录 Apache-2.0**（PCL.Core 明确 Apache-2.0） |
| 默认分支 | `dev`（另有大量 `codex/*` 安全修复分支） |
| CI | build-test.yml、release-publish.yml、reusable-build.yml、mirrorchyan_uploading.yml 等 |
| 最新版本 | v2.15.1-beta.1（2026-09-04）；正式版 v2.15.0（2026-07-14） |
| 发布物 | PCL2_CE_Release/Beta_x64.exe、ARM64.exe（含 .asc 签名） |
| 发行量 | 30 个 release，累计下载 348,942 次 |
| 结构 | `PCL.Core/`（App、IO、Link、Logging、Minecraft、Model、UI、Utils、ViewModel）、`PCL.Core.SourceGenerators/`、`PCL.Core.Test/`、`Plain Craft Launcher 2/`（XAML 页面：PageDownload、PageHomepage、PageInstance、PageLaunch、PageSetup、PageTools…，Modules：Base、Event、Minecraft、Network、UI、Updates）、`.slnx` 解决方案 |

关键事实：
- 2.15.0 版本说明：**全面移除 Visual Basic 代码**，重构核心组件，升级 .NET 10 运行时，首次加入多语言支持。
- 需 .NET 10 Desktop Runtime；Win10 1809+ 完整支持；Win7 不支持；macOS/Linux 仅可交叉编译。
- 社区版版本号与官方主线不严格对应，不要把 CE 问题反馈给官方。

## 三、两者关系与差异

| 维度 | PCL | PCL-CE |
|---|---|---|
| 定位 | 官方主线 | 社区二次开发版 |
| 语言 | VB.NET 为主（+ 少量 C# 库 PCLCS） | 纯 C#（2.15.0 起无 VB） |
| 运行时 | .NET Framework 4.8 | .NET 10 |
| 代码可复用性 | 低（自定义许可，几乎全仓库受限） | 中（PCL.Core 等目录 Apache-2.0，可直接复用；UI 目录仍受限） |
| MC 下载 | 2.99.0.0 起移除 | 保留下载/资源安装并持续改进（自动装前置、区分必需/可选依赖） |
| 界面语言 | 中文为主 | 多语言 |
| 发布节奏 | 不定期，发布同步源码 | 频繁 beta/正式版 |

## 四、对 Dream Launchpad Launcher 的影响（结论）

1. **不能直接抄 PCL 的 UI/启动代码并继续叫 Dream Launchpad Launcher**：PCL 自定义许可要求重度使用时名称必须以 `Plain Craft Launcher (PCL)` 开头、蓝色主色相、署名龙腾猫跃、实现 McLaunchPrecheck 劝导等。要独立命名，只能「轻度使用」（参考极小部分）或完全自研。
2. **可合法复用的是 Apache-2.0 部分**：`Meloong-Git/MeloongCore`（C# .NET 通用库）与 `PCL-CE/PCL.Core`（含 Minecraft/下载/IO 等核心逻辑），需保留 Apache-2.0 许可与 NOTICE。
3. **参考架构**：CE 的 `PCL.Core` + WPF 页面分层是清晰的三层结构（核心库 / UI 页面 / 模块），PCL 的 `PageXxxLeft/Right` 双栏页面布局也可借鉴设计思路（不复制代码）。
4. **技术选型建议**：若只做 Windows，C# + WPF + .NET 10 与 CE 一致，生态最成熟；若要多平台，WPF 不适合，需 Avalonia 等（CE 也只支持交叉编译，不提供 macOS/Linux 运行）。

## 五、参考链接
- https://github.com/Meloong-Git/PCL
- https://github.com/Meloong-Git/MeloongCore
- https://github.com/PCL-Community/PCL-CE
- https://meloong.com/pcl
- https://ce.pclc.cc
