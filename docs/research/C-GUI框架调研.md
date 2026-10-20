# C 语言跨平台 GUI 框架调研（MC 启动器选型）

- 日期：2026-10-20
- 目标平台：Windows（含 Windows 7）、Linux x86_64、Linux arm64
- 约束：UI 现代（圆角/动画/自定义主题），安装包体积小
- 核实方式：GitHub API（`gh api`）+ 各项目官网/GitLab/SourceForge 直接抓取；GTK4 Windows 体积为**实测**（下载 gvsbuild 发布 zip 的中央目录，统计 `bin/*.dll` 与 `share/` 体积）
- 结论摘要：**C 生态里没有"Win7 + 现代 UI + 小体积"三者兼得的路**。首选 GTK4（放弃 Win7），备选 FLTK（保 Win7，牺牲现代观感）。

---

## 1. 总表

| 框架 | C 调用方式 | 平台覆盖 | 体积量级 | UI 现代度 | 许可 | 最新版（日期） | 近 3 月活跃 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **GTK4** | 原生 C（GObject C API） | Linux ✓ / Win10+ ✓（Win7 ✗）/ macOS ✓；Linux arm64 ✓ | Win 实测：核心 DLL 42 个 **45.3 MB**、`bin/*.dll` 67 个 120.7 MB、`share/` 70.5 MB | 最高（libadwaita、CSS、动画、圆角） | LGPL-2.1+ | 4.24.1（2026-10-01） | ✓ |
| **FLTK 1.4** | C++ 核心；C 绑定 `cfltk` 非官方 | Win（Win32，含 Win7）/ Linux（X11/Wayland）/ macOS；Debian arm64 ✓ | 单 DLL 约 1–3 MB | 低（自绘控件，方案/主题能力弱） | LGPL-2.0 + 例外 | 1.4.5（2026-04-25） | ✓ |
| **IUP 3.32** | 原生 C | Win（原生控件）/ Linux（GTK3）/ macOS；arm64 未验证 | Win 端几 MB；Linux 端被迫拖 GTK3（≈45 MB） | 中低（系统原生控件，Canvas 需自绘） | MIT | 3.32（2025-01） | 弱（发版慢） |
| **Nuklear** | 原生 C，单头文件 | 无平台层（需 GLFW/SDL/GL 后端） | <1 MB | 低–中（自绘，圆角靠近似） | Public Domain / MIT | 4.13.3（2026-05-05） | ✓ |
| **LVGL 9.6** | 原生 C | 嵌入式为主；桌面走 SDL/GLFW 驱动 | 1–3 MB | 中（自带主题+动画） | MIT | 9.6.0（2026-09-16） | ✓ |
| **Dear ImGui** | C++ 核心；`cimgui` 自动生成 C API | 同 ImGui 后端（Win/DX11/OpenGL） | 1–2 MB | 低（工具风，布局/滚动条不适合消费级） | MIT | imgui 1.93.0（2026-10-14）；cimgui 无 release（2018 起） | ✓（cimgui master 追到 imgui 1.92.9b） |
| **Tcl/Tk 9.1** | 原生 C（Tcl C API + Tk 控件） | Win ✓（Win32）/ Linux / macOS | 官方 Windows 单文件 Wish 包 **3.5 MB** | 中（ttk 可换主题，布局系统老旧） | BSD 系 | 9.1.0（2026-09-30） | ✓ |
| **EFL** | 原生 C | Linux ✓；**Windows 二进制停在 2023-07-22**（win10 命名） | Linux 分发包 161 MB（源码 tar） | 中高（Evas/Edje 主题、动画） | LGPL/BSD 混合 | 1.28.1（2025-03-17） | Linux ✓ / Windows 停更 |
| **libui** | 原生 C | Win/Linux/macOS 原生控件 | 小 | 低 | MIT | 默认分支最后提交 **2021-01-02** → 已死 | ✗ |
| **LCUI** | 原生 C | Win / Linux（CSS 式布局） | 未验证 | 中 | MIT | v2.2.0（2021-06-01），v3 仍 alpha | ✓（仓库活跃） |
| **raylib 6.2** | 原生 C | Win/Linux/ARM/Android/Web | 1–3 MB | 低–中（游戏自绘，raygui 立即模式） | Zlib | 6.2（2026-10-15） | ✓ |
| **SDL3** | 原生 C | Win（官方称仍支持 XP 起）/Linux/macOS | 1–2 MB | 无控件库（只有窗口+2D 绘制） | Zlib | 3.4.18（2026-10-02） | ✓ |
| **WebView2** | 无官方 C API（COM/C++） | 仅 Windows；**Win7 支持随 Edge 109 结束（2023-01-12）** | 依赖系统预装 Runtime | 最高（网页） | 专有 | 随 Edge 走 | ✓ |
| **CEF** | 有 C API（`cef_capi.h`） | Win/macOS/Linux；Chromium 早已不支持 Win7 | 约 150 MB+ | 高（网页） | BSD | 随 Chromium 走（未逐版核实） | ✓ |
| **webui** | 原生 C | Win（WebView2）/ Linux（GTK WebKit2） | 小（壳）+ 系统 WebView | 高（网页） | MIT | 仓库活跃（2026-10-14 有提交） | ✓ |

---

## 2. 关键核实结论（逐条）

### GTK4 在 Windows 上发给终端用户现实吗？
- **不现实，但它是 C 生态唯一"现代 UI"的正经货**。
- 官方推荐的 Windows 构建来自 gvsbuild（MSYS2 也可）：`GTK4_Gvsbuild_2026.8.0_x64.zip` **287 MB（300,885,620 字节）**，含 dev 头文件/工具。来源：<https://github.com/wingtk/gvsbuild/releases/tag/2026.8.0>
- **实测运行时体积**（解析该 zip 中央目录）：
  - `bin/*.dll` 共 67 个 = **120.7 MB**；其中 GTK4 核心依赖集合 42 个 = **45.3 MB**（`gtk-4-1.dll` 单个 10.8 MB，`gio-2.0-0.dll` 2.3 MB，`gdk-pixbuf`/pango/cairo/harfbuzz 若干）
  - `bin/rsvg-2-2.dll` 30.6 MB、`libprotobuf.dll` 15.0 MB、`gtksourceview-5-0.dll` 3.0 MB（这些是整套构建附带，可裁）
  - `share/`（主题、图标、翻译、gdk-pixbuf 加载器）= **70.5 MB**（可裁到 10–30 MB，Adwaita 图标+主题必需）
  - 结论：**最小可分发的 GTK4 Windows 运行时约 60–80 MB（未压缩）/ 25–35 MB（压缩）**，DLL 数量在 30–40 个。
- **Windows 7 明确不支持**：GTK 的 `meson.build` 第 82–83 行给 C 编译统一加 `-DWINVER=_WIN32_WINNT_WIN10 -D_WIN32_WINNT=_WIN32_WINNT_WIN10`。来源：<https://gitlab.gnome.org/GNOME/gtk/-/blob/main/meson.build>
- **Linux arm64 有官方包**：Debian stable `libgtk-4-1 4.18.6+ds-2` 提供 amd64/arm64/armhf/i386。来源：<https://packages.debian.org/stable/libs/libgtk-4-1>
- 最新版 4.24.1（2026-10-01）。来源：<https://gitlab.gnome.org/GNOME/gtk/-/releases>

### EFL / Enlightenment 在 Windows 上的现状
- **Windows 侧事实上停更**。官方 Windows 安装包目录最后两个产物是 `efl-x86_64-win10-1.26.exe`（28 MB）与 `ewpi-x86_64-win10-1.1.exe`（70 MB），时间戳均为 **2023-07-22**，文件名直接写死 `win10`（即不面向 Win7/8）。来源：<https://download.enlightenment.org/rel/win/efl/>
- 还能编译：官方文档仍给出 MSYS2 + Ewpi 自建流程。来源：<https://www.enlightenment.org/docs/distros/windows-start>
- Linux 侧仍发版：efl-1.28.1.tar.xz，**2025-03-17**，161 MB。来源：<https://download.enlightenment.org/rel/libs/efl/>
- 仓库镜像仍在动（2026-10-20 有 push），但那是 Linux 侧。来源：<https://github.com/Enlightenment/efl>

### libui 是否已停止维护
- 是。默认分支（master）最后一次提交 **2021-01-02**（"Merge pull request #521 from kojix2/patch-1"），tag 最高 `alpha4.1`，从未发过 1.0。来源：<https://api.github.com/repos/andlabs/libui/commits>、<https://github.com/andlabs/libui/tags>
- 「pushed_at 2024-05」是别的分支/PR，不代表维护。

### IUP 的 Win/Linux 支持与体积
- 最新版 3.32（官网首页 "Version 3.32"）。来源：<https://www.tecgraf.puc-rio.br/iup/>
- 三平台：Windows 用原生 Win32 控件，Linux 走 GTK3（意味着 **Linux 端体积 = IUP + GTK3 ≈ 45 MB+**），macOS 原生。源码/二进制在 SourceForge 分目录发布（Windows Libraries / Linux Libraries / Tools Executables）。来源：<https://sourceforge.net/projects/iup/files/3.32/>
- 中文生态：有零散教程（知乎《用 IUP 开发 GUI 图形界面》系列、GitHub `fangcun010/IUP-Tutorial-Chinese`），**量级远小于 GTK/FLTK/Qt**，遇到问题基本只能读官方 CHM。
- 体积未逐包实测（SourceForge 子目录体积抓取失败），按"Win32 薄封装"判断为**几 MB 量级（估算）**；Linux 端体积取决于 GTK3。

### Nuklear 能不能撑起应用级 UI
- 定位就是"极简、可嵌入"：README 原话 "minimal-state, immediate-mode graphical user interface toolkit … designed as a simple embeddable user interface"，**无默认渲染后端、无窗口/输入抽象**（要自己接 GLFW/SDL）。来源：<https://github.com/Immediate-Mode-UI/Nuklear>
- 控件覆盖够（列表/滚动/输入框/图片/表格都有），但**立即模式 + 无布局引擎**：虚拟列表、超长滚动、输入法（中文 IME）、无障碍都要自己兜。做 MC 启动器这种"列表+封面图+表单"能用，但等于自建半个 UI 框架。
- 活跃：4.13.3（2026-05-05），2026-10 仍有提交。

### Tcl/Tk 的 Windows 分发体积与外观
- 官方 Windows 单文件构建 `Wish.9.1.0.Windows.single-file.build.snapshot.zip` 仅 **3,533,268 字节（3.4 MB）**；Linux 版 3.3 MB，源码包 5.3 MB。来源：<https://github.com/tcltk/tk/releases/tag/core-9-1-0>
- 外观：ttk 主题可替换（Sun Valley / Arc / breeze 等第三方主题），能做出"接近现代"的观感，但 **Tk 的几何管理器（pack/grid）与事件模型是 1990 年代设计**，复杂界面（卡片、动效、高 DPI 缩放）代价高，动画基本靠 `after` 手搓。
- 9.1.0 发布 2026-09-30，Win/Linux 均活跃。

### Dear ImGui + cimgui 适不适合消费级应用
- **不适合**。ImGui 自述面向工具/调试（FAQ 有专条 "What is the difference between Dear ImGui and traditional UI toolkits?"）。来源：<https://github.com/ocornut/imgui/blob/master/docs/FAQ.md>
- C 绑定 `cimgui` 仍是"薄自动生成层"，仓库活跃（2026-10-18），但**最近 release 是 2018-01-02 的 v1.53.1**，只能用 master 且与 imgui 版本手工对齐（当前追 imgui 1.92.9b）。来源：<https://github.com/cimgui/cimgui>
- 文本输入/IME、滚动条、无障碍、HiDPI 在 ImGui 里都是弱项。

### 其他必须知道的
- **SDL3 无控件**，只有窗口/输入/2D；官方 README 明确"back to Windows XP"仍支持桌面 Windows。来源：<https://github.com/libsdl-org/SDL/blob/main/docs/README-windows.md>
- **raylib** 官方支持 Windows/Linux/macOS/RPI/Android/HTML5；控件只有 raygui（立即模式）。来源：<https://github.com/raysan5/raylib>
- **FLTK** 官方支持 Windows（Win32）/X11/Wayland/macOS；Debian `libfltk1.3t64` 有 arm64。许可为 LGPL + 静态链接例外（COPYING 原文："GNU Library General Public License (LGPL) with the following exceptions"）。注意 **FLTK 核心是 C++**，纯 C 需要 `cfltk`/`fltk-c` 这类**非官方**绑定。
- **WebView2**：Edge/WebView2 对 Windows 7/8.1 的支持在 **Edge 109 结束（2023-01-12）**，且官方没有 C API（COM/C++/C#/WinRT）。来源：<https://learn.microsoft.com/en-us/deployedge/microsoft-edge-supported-operating-systems>
- **CEF** 提供 C API（`cef_capi.h`），但 Chromium 早已不支持 Win7，体积 150 MB+ 量级（未逐版核实具体包体）。
- **webui**（C，MIT，4.6k star）本质是 WebView 壳：Windows 用 WebView2、Linux 用 GTK WebKit2 —— 直接继承两者的限制（Win7 不可用；Linux 需 webkit2gtk 依赖）。来源：<https://github.com/webui-dev/webui>
- **LCUI**（C，MIT）仓库仍在动（2026-10-19），但正式版停在 v2.2.0（2021-06-01），v3 只有 alpha；中文项目、文档弱、生态小，不建议押注。来源：<https://github.com/lc-soft/LCUI>

---

## 3. 结论

**首选：GTK4 + libadwaita（纯 C）**
- 代价：**放弃 Windows 7**（GTK 编译期就锁 Win10 API）；Windows 安装包 +25–35 MB 压缩体积、30–40 个 DLL（可用 `gvsbuild` 裁剪 + 只带 Adwaita）；Linux 端零成本（发行版自带，arm64 官方有包）。
- 收益：C 生态里唯一"开箱即现代"的 UI（圆角、动画、CSS 主题、无障碍），且是原生 C API，不经过绑定层。

**备选：FLTK 1.4（cfltk 绑定）**
- 代价：UI 观感停留在 2010 年代，圆角/动画/自定义主题都要自绘；C 绑定非官方（升级跟随靠社区）；中文生态一般。
- 收益：**Win7 能跑**、体积 1–3 MB、许可证宽松（LGPL + 静态链接例外）、单 DLL 分发、Linux arm64 有 Debian 包。

**如果 Win7 + 现代 UI 是硬需求：C 没有现实方案。** 最接近的三条替代：
1. **C++ / Qt 6**（Win7 需 Qt 5.15，Qt 6 要求 Win10）—— 现代 UI、体积 30–60 MB；
2. **C + WebView 壳**（webui / 自写 WebView2 宿主）—— 网页 UI 最省事，但 Win7 上限是"固定版本 WebView2 109 或降级到 MSHTML"；
3. **Rust + Tauri / C++ + 系统 WebView** —— 体积 8–15 MB、UI 任意，代价是引入 Web 技术栈。

---

## 4. 来源清单

- GTK 发布：<https://gitlab.gnome.org/GNOME/gtk/-/releases>（4.24.1, 2026-10-01）
- GTK Windows 最低 API：<https://gitlab.gnome.org/GNOME/gtk/-/blob/main/meson.build>（`_WIN32_WINNT_WIN10`）
- GTK4 Windows 构建包：<https://github.com/wingtk/gvsbuild/releases/tag/2026.8.0>（GTK4 zip 300,885,620 B, 2026-08-08）
- GTK4 Linux arm64 包：<https://packages.debian.org/stable/libs/libgtk-4-1>（4.18.6+ds-2, amd64/arm64/armhf/i386）
- FLTK：<https://github.com/fltk/fltk/releases/tag/release-1.4.5>（2026-04-25）、许可 <https://github.com/fltk/fltk/blob/master/COPYING>、Debian arm64 <https://packages.debian.org/stable/libs/libfltk1.3t64>
- IUP：<https://www.tecgraf.puc-rio.br/iup/>（Version 3.32）、<https://sourceforge.net/projects/iup/files/3.32/>、<https://iup.sourceforge.net/>
- Nuklear：<https://github.com/Immediate-Mode-UI/Nuklear>（v4.13.3, 2026-05-05）
- LVGL：<https://github.com/lvgl/lvgl/releases/tag/v9.6.0>（2026-09-16）
- Dear ImGui：<https://github.com/ocornut/imgui/releases/tag/v1.93.0>（2026-10-14）、cimgui <https://github.com/cimgui/cimgui>（最后 release v1.53.1, 2018-01-02）
- Tcl/Tk：<https://github.com/tcltk/tk/releases/tag/core-9-1-0>（2026-09-30；Windows 单文件 3,533,268 B）
- EFL：<https://download.enlightenment.org/rel/libs/efl/>（1.28.1, 2025-03-17）、Windows 产物 <https://download.enlightenment.org/rel/win/efl/>（1.26, 2023-07-22）、Windows 构建文档 <https://www.enlightenment.org/docs/distros/windows-start>
- libui：<https://github.com/andlabs/libui>（默认分支最后提交 2021-01-02）
- LCUI：<https://github.com/lc-soft/LCUI>（v2.2.0, 2021-06-01）
- raylib：<https://github.com/raysan5/raylib/releases/tag/6.2>（2026-10-15）
- SDL3：<https://github.com/libsdl-org/SDL/releases/tag/release-3.4.18>（2026-10-02）、Windows 支持范围 <https://github.com/libsdl-org/SDL/blob/main/docs/README-windows.md>
- WebView2 / Edge 操作系统支持：<https://learn.microsoft.com/en-us/deployedge/microsoft-edge-supported-operating-systems>（Win7 支持在 Edge 109 结束，2023-01-12）
- webui：<https://github.com/webui-dev/webui>

## 5. 未验证 / 需补测

- IUP 的 Windows/Linux 预编译包**具体字节数**（SourceForge 子目录元数据抓取失败），本文按架构估算为"Win 几 MB / Linux ≈ GTK3 45 MB+"。
- IUP 在 Linux arm64 上是否有官方二进制（未查到）。
- CEF 最新 Windows 包体精确大小（只确认"百 MB 量级"）。
- Tk 9.1 在 Windows 7 上的实际运行（Tk 走 Win32 API，理论上可用，未实测）。
- LCUI v3 的成熟度（只有 alpha tag，未试编译）。
- GTK4 Windows 包裁到"最小可运行"的确切 DLL 清单（本文给的是核心依赖集合 42 个 / 45.3 MB 的上界）。
