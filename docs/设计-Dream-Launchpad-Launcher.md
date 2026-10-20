# Dream Launchpad Launcher 设计文档

版本：v0.1（2026-10-20）
状态：设计定稿，待实现

---

## 1. 项目目标

从零实现一个 Windows 平台的 Minecraft: Java Edition 启动器，代号 **DLP（Dream Launchpad Launcher）**。

**要解决的问题**
- 一键安装任意正式版 / 快照版，自动补齐 Java 运行库、资源文件、依赖库
- 多实例隔离：每个整合包独立 mods / saves / 配置，互不污染
- 离线可用：官方源不可达时自动切换镜像；已下载资源可复用
- 启动失败可诊断：完整日志、崩溃报告、错误码

**非目标（v1 不做）**
- 不做服务器联机 / 大厅
- 不做基岩版
- 不内置 MC 下载的破解或绕过正版验证
- 不复制 PCL 的受限代码（见第 14 章）

---

## 2. 设计原则

1. **核心与 UI 分离**：全部逻辑放在无 UI 依赖的类库，CLI 与 WPF 共用；核心可单元测试。
2. **一切可校验**：所有下载走 SHA1 校验，失败即重下，绝不静默使用坏文件。
3. **安装幂等**：重复安装同一版本只补缺失文件；安装过程可中断、可恢复。
4. **网络可替换**：下载源抽象成 IDownloadSource，官方源 / BMCLAPI / 自建镜像一键切换。
5. **无遥测**：不向任何服务器上报用户数据；日志只留本地。
6. **最小依赖**：核心库零第三方依赖，UI 只引入 MVVM 工具库。

---

## 3. 技术选型

| 层 | 选型 | 说明 |
|---|---|---|
| 语言 / 运行时 | C# 13 / .NET 10 | 本机已装 SDK 10.0.402 |
| UI | WPF + MVVM（CommunityToolkit.Mvvm） | Windows 原生，启动快，控件成熟 |
| JSON | System.Text.Json（source generator） | 零反射、AOT 友好 |
| HTTP | HttpClient + SocketsHttpHandler | 连接池、并发、代理支持 |
| 存储 | JSON 文件 + 文件系统 | v1 不引入数据库；资产索引按需加载 |
| 日志 | 自研轻量 Logger | 滚动文件、线程安全 |
| 测试 | xUnit | 核心逻辑全覆盖 |
| 打包 | dotnet publish -r win-x64 --self-contained 单文件 | 用户免装 .NET |

**为什么不用 Web/Electron**：启动器需要频繁访问本地文件、注册表、进程管理，WPF 直接调用成本最低；单文件发布体积可控（约 70–90 MB），且不依赖外部运行时。

**为什么不用 Avalonia**：v1 只做 Windows；跨平台留到 v2，届时把 DLP.Core（netstandard 兼容）直接复用。

---

## 4. 总体架构

~~~
┌──────────────────────────────────────────────────────────┐
│  DLP.App (WPF)                                           │
│  Views (XAML)  ←→  ViewModels  ←→  Application Services  │
├──────────────────────────────────────────────────────────┤
│  DLP.Core (net10.0, 无 UI 依赖)                            │
│  ┌────────────┬────────────┬────────────┬───────────────┐ │
│  │ Minecraft  │ Install    │ Launch     │ Auth          │ │
│  │ 领域模型    │ 安装编排    │ 进程/参数   │ 离线/微软      │ │
│  ├────────────┼────────────┼────────────┼───────────────┤ │
│  │ Download   │ Java       │ Instance   │ Version       │ │
│  │ 队列/校验   │ 运行时管理  │ 实例隔离    │ 清单/继承      │ │
│  └────────────┴────────────┴────────────┴───────────────┘ │
├──────────────────────────────────────────────────────────┤
│  Infrastructure: Http / FileSystem / Process / Crypto /  │
│                  Logging / Zip / Config                  │
└──────────────────────────────────────────────────────────┘
~~~

**解决方案结构**

~~~
DreamLaunchpad.sln
├─ src/
│  ├─ DLP.Core/            # 领域 + 服务（无 WPF）
│  │  ├─ Minecraft/        # VersionJson、Library、Rule、AssetIndex
│  │  ├─ Install/          # 安装编排、安装清单
│  │  ├─ Download/         # 下载队列、校验、镜像
│  │  ├─ Java/             # 扫描、识别、Mojang JRE 下载
│  │  ├─ Auth/             # 离线 UUID、Microsoft OAuth
│  │  ├─ Launch/           # classpath、参数、进程
│  │  ├─ Instance/         # 实例模型、目录隔离
│  │  └─ Infrastructure/   # Http、文件、进程、日志、加密
│  ├─ DLP.Cli/             # 命令行（调试 / 自动化测试）
│  └─ DLP.App/             # WPF 界面
└─ tests/
   └─ DLP.Core.Tests/
~~~

依赖方向：DLP.App → DLP.Core；DLP.Cli → DLP.Core；Core 内部按命名空间分层，不反向依赖。

---

## 5. 数据目录布局

默认根目录 %APPDATA%\DreamLaunchpad；若 exe 同级存在 portable.txt 则使用 exe 目录（便携模式）。

~~~
DreamLaunchpad/
├─ config.json                  # 全局设置
├─ accounts.json                # 账号（token 用 DPAPI 加密）
├─ instances/
│  └─ <实例名>/
│     ├─ instance.json          # 实例配置
│     ├─ minecraft/             # 游戏目录（隔离）
│     │  ├─ mods/ saves/ resourcepacks/ shaderpacks/ config/
│     │  └─ options.txt
│     └─ logs/
├─ versions/
│  └─ <版本ID>/
│     ├─ <版本ID>.json          # 版本清单（含继承链合并结果另存 merged.json）
│     ├─ <版本ID>.jar           # client jar
│     ├─ natives/               # 解压后的 native 库
│     └─ install.json           # 安装回执（文件 → sha1，用于修复）
├─ libraries/                   # 共享库，Maven 布局
├─ assets/
│  ├─ indexes/<索引ID>.json
│  ├─ objects/<前2位>/<剩余38位>
│  └─ virtual/legacy/           # 1.7 以前的虚拟资源
├─ runtime/                     # Mojang JRE
│  └─ <组件名>/windows-x64/<版本>/
├─ cache/
│  ├─ version_manifest_v2.json
│  └─ downloads/                # 未完成下载（.part）
└─ logs/
   ├─ launcher-2026-10-20.log
   └─ crash-<时间>.zip
~~~

**隔离策略**：libraries/assets/versions/runtime 全局共享；mods/saves/config 按实例隔离。实例可复用同一版本 JSON，避免重复下载。

---

## 6. 核心数据模型

~~~csharp
// 版本清单（version_manifest_v2.json）
record VersionManifest(LatestVersion Latest, IReadOnlyList<VersionEntry> Versions);
record LatestVersion(string Release, string Snapshot);
record VersionEntry(string Id, string Type, string Url, string Sha1,
                    DateTimeOffset ReleaseTime, int ComplianceLevel);

// 版本 JSON（version/<id>.json，字段可能被 inheritsFrom 覆盖/合并）
class VersionJson {
    string Id;
    string? InheritsFrom;
    string MainClass;
    string? Assets;                 // 资源索引 ID
    AssetIndexRef? AssetIndex;
    VersionDownloads Downloads;     // client / server
    List<Library> Libraries;
    List<Argument>? Arguments;      // 新格式（jvm + game）
    string? MinecraftArguments;     // 旧格式（1.12 及以前）
    JavaVersion? JavaVersion;
    LoggingConfig? Logging;
    string Type;
    int? MinimumLauncherVersion;
}

record AssetIndexRef(string Id, string Sha1, long Size, long TotalSize, string Url);
record Library(string Name, LibraryDownloads? Downloads, Dictionary<string,string>? Natives,
               List<Rule>? Rules, ExtractRule? Extract);
record LibraryDownloads(LibraryArtifact? Artifact, Dictionary<string, LibraryArtifact>? Classifiers);
record LibraryArtifact(string Path, string Sha1, long Size, string Url);
record Rule(string Action, OsRule? Os, Dictionary<string,bool>? Features);
record OsRule(string? Name, string? Arch, string? Version);
record Argument(string? Value, List<Rule>? Rules);

record AssetIndex(string Id, Dictionary<string, AssetObject> Objects);
record AssetObject(string Hash, long Size);

record JavaVersion(string Component, int MajorVersion);   // component: java-runtime-epsilon
record LoggingConfig(LoggingFile Client);
record LoggingFile(string Argument, string Id, string Sha1, long Size, string Url);

// 账号
record Account(string Id, string Username, AccountType Type, string Uuid,
               string? AccessToken, DateTimeOffset? ExpiresAt, string? RefreshToken,
               string? Xuid, string? SkinUrl);
enum AccountType { Offline, Microsoft }

// 实例
class Instance {
    string Name;
    string VersionId;
    string? JavaPath;        // 为空则自动选择
    int MemoryMb;
    List<string> JvmArgs;
    string? Resolution;      // "854x480"
    bool IsIsolated;
}
~~~

**GAV 坐标解析**：group:artifact:version[:classifier][@ext]，Maven 路径 = group.replace('.','/')/artifact/version/artifact-version[-classifier].ext。

---

## 7. 关键流程

### 7.1 版本清单同步

~~~
GET https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
  Header: If-None-Match: <缓存 ETag>
缓存 TTL 1 小时；304 直接读本地；失败回退镜像（BMCLAPI：https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json）
~~~

镜像切换规则：官方源连续 2 次失败（超时 8s / 5xx）→ 标记降级 10 分钟 → 用镜像；镜像 URL 与官方 URL 路径一一对应。

### 7.2 安装流程（原版）

~~~
InstallAsync(versionId, progress):
  1. 解析版本：manifest 找到 entry → 下载 version JSON → 校验 sha1
  2. 处理 inheritsFrom：深度优先合并父版本（libraries 追加、arguments 拼接、字段子覆盖父）
  3. 下载 client jar：versions/<id>/<id>.jar（校验 sha1）
  4. 处理 libraries：
       for lib in libraries:
         if !EvaluateRules(lib.Rules): continue
         if lib.Natives != null:
             classifier = lib.Natives["windows"]        // 例如 natives-windows
             artifact = lib.Downloads.Classifiers[classifier]
             download → 解压到 versions/<id>/natives（排除 META-INF/、*.git 等）
         else:
             download lib.Downloads.Artifact → libraries/<maven路径>
  5. 资源索引：下载 assetIndex → assets/indexes/<id>.json
  6. 资源对象：对 objects 去重后并发下载 → assets/objects/<hash[0:2]>/<hash>
       （旧版本 assets=legacy 时建立 virtual/legacy 硬链接或复制）
  7. 日志配置：下载 logging.client.file → versions/<id>/log4j.xml
  8. Java：若 javaVersion.component 对应 runtime 未安装 → 下载 Mojang JRE（见 7.3）
  9. 写 install.json 回执（相对路径 + sha1 + size）
 10. 标记完成；失败则回滚到该版本目录的 .installing 临时状态
~~~

进度模型：totalBytes / downloadedBytes / 当前阶段 / 当前文件，阶段权重：清单 2%、client 8%、库 20%、资源 60%、Java 10%。

下载并发默认 8（可配 1–32）；失败重试 3 次，指数退避 1s/2s/4s；同一文件并发去重（ConcurrentDictionary<string, Task>）。

### 7.3 Java 运行时管理

**扫描顺序**
1. 实例/全局手动指定路径
2. JAVA_HOME
3. 启动器自带 runtime/*
4. 常见安装目录：C:\Program Files\Java\*、C:\Program Files\Eclipse Adoptium\*、%LOCALAPPDATA%\Programs\Eclipse Adoptium\*
5. PATH 中的 java.exe

**识别**：读取 release 文件（JAVA_VERSION="25.0.1"、OS_ARCH="x86_64"）；没有 release 文件时回退执行 java -XshowSettings:properties -version 解析。

**选择**：版本 JSON 的 javaVersion.majorVersion 为最低要求；优先选择主版本相等且 64 位；1.12 及以前用 Java 8 最稳（可选“兼容模式”）。

**Mojang JRE 下载**
~~~
GET https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json
→ all["windows-x64"]["java-runtime-epsilon"][0].manifest.url
→ 该 manifest 的 files: 每个文件 {type:file|directory|link, downloads:{raw:{sha1,size,url}}, executable}
→ 逐文件下载到 runtime/<component>/windows-x64/<version>/
→ 校验；记录安装信息
~~~
组件对应：java-runtime-alpha=8、beta=11、gamma=17、delta=21、epsilon=25。

### 7.4 账号与认证

**离线登录**：UUID = MD5("OfflinePlayer:" + name) 按 UUID v3 格式化；accessToken 填 "0"；user_type=legacy。仅用于离线游戏，不能进正版服务器。

**微软登录（OAuth 2.0 Device Code）**
~~~
1. POST https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode
   body: client_id=<自有 Azure 应用>, scope="XboxLive.signin offline_access"
   → user_code + verification_uri（向用户展示）
2. 轮询 POST .../oauth2/v2.0/token  grant_type=urn:ietf:params:oauth:grant-type:device_code
   → access_token + refresh_token
3. POST https://user.auth.xboxlive.com/user/authenticate
   { Properties: { AuthMethod:"RPS", SiteName:"user.auth.xboxlive.com", RpsTicket:"d=<ms_token>" },
     RelyingParty:"http://auth.xboxlive.com", TokenType:"JWT" } → xbl_token
4. POST https://xsts.auth.xboxlive.com/xsts/authorize
   { Properties:{ SandboxId:"RETAIL", UserTokens:[xbl_token] }, RelyingParty:"rp://api.minecraftservices.com/", TokenType:"JWT" }
   → xsts_token + userhash（注意 2148916233/2148916238 等错误码：无 Xbox 账号 / 儿童账号）
5. POST https://api.minecraftservices.com/authentication/login_with_xbox
   { identityToken: "XBL3.0 x=<userhash>;<xsts_token>" } → mc_access_token（默认 24h）
6. GET https://api.minecraftservices.com/minecraft/profile
   → id（无横线 UUID）、name、skins
~~~
Token 刷新：Microsoft token 过期用 refresh_token 换新；Minecraft token 过期重走 3–6 步。accounts.json 中 token 用 DPAPI（ProtectedData.Protect，CurrentUser scope）加密。

### 7.5 启动流程

~~~
LaunchAsync(instance, account):
  1. 合并版本 JSON（含 inheritsFrom）
  2. 构建 classpath：
       libraries 中通过规则且非 native 的 artifact 路径（按 JSON 顺序）
       + versions/<id>/<id>.jar
  3. 解压 natives（若未解压或 jar 变更）
  4. 生成变量表：
       auth_player_name, version_name, game_directory, assets_root,
       assets_index_name, auth_uuid, auth_access_token, user_type,
       version_type, user_properties, clientid, auth_xuid,
       natives_directory, launcher_name, launcher_version, classpath
  5. JVM 参数：
       内存：-Xms<mem/4>M -Xmx<mem>M（默认 2048，可配）
       -Djava.library.path=${natives_directory}
       -Dminecraft.launcher.brand=DLP -Dminecraft.launcher.version=<ver>
       -cp ${classpath}（新版 JSON 的 jvm 参数里已含）
       日志：-Dlog4j.configurationFile=<log4j.xml>（若 logging 存在）
       用户自定义 JVM 参数追加在 -cp 之前
  6. 游戏参数：按 arguments.game 的规则过滤（features 为空集），替换 ${...} 占位符
      旧版本用 minecraftArguments 按空格拆分后替换
  7. 进程：
       ProcessStartInfo { FileName=javaPath, WorkingDirectory=实例 minecraft 目录,
                          UseShellExecute=false, RedirectStandardOutput/Error=true }
       逐行读取 stdout/stderr → 日志（同时写 launcher log 与 latest.log）
  8. 等待退出：记录退出码；非 0 时收集 crash-reports/hs_err_pid*.log 生成诊断包
~~~

**注入安全**：绝不拼接命令行字符串；使用 ArgumentList 逐个传参；路径含空格、中文、引号均安全。

### 7.6 版本继承与 Mod 加载器

inheritsFrom 链：子版本覆盖 mainClass、assets、downloads；libraries 追加（同名 artifact 以子为准）；arguments 拼接。

- **Fabric**：https://meta.fabricmc.net/v2/versions/loader/<mc> 选 loader → 生成 profile JSON（含 inheritsFrom、libraries、mainClass net.fabricmc.loader.impl.launch.knot.KnotClient）→ 走同一安装管线。
- **Quilt**：https://meta.quiltmc.org/v3/versions/loader/<mc>，同 Fabric。
- **Forge / NeoForge**：下载 installer jar，解析 install_profile.json 得到 version JSON 与 processors；执行 processors 需要 Java 与网络，v1 后期实现（或用官方 installer 命令行 --installClient）。

### 7.7 实例管理

- 创建实例：选择版本 → 生成 instances/<name>/ + instance.json + minecraft/ 目录
- 启动时工作目录指向实例的 minecraft/，--gameDir 同值
- 复制实例、导出整合包（zip：instance.json + mods + config + 版本清单）后续实现
- 实例重命名时同步更新 instance.json 与目录名（先改配置再改目录，失败可回滚）

### 7.8 下载引擎

~~~
DownloadManager:
  队列（按优先级：install > 用户操作 > 后台预取）
  并发控制：SemaphoreSlim(8)
  每文件：若目标存在且 size+sha1 匹配 → 跳过
  支持 Range 断点续传（.part 文件）
  失败：重试 3 次 → 记录到 failed.json → 结束后汇总提示
  镜像：同一路径按当前源改写 host；校验始终用官方 sha1
~~~

### 7.9 启动器更新

GET https://api.github.com/repos/<org>/<repo>/releases/latest → 比较版本 → 下载 exe → 校验 → 替换并重启（需管理员权限时提示）。v1 最后实现。

---

## 8. UI 设计

**主窗口**：无边框 + 自绘标题栏（可拖动、最小化/最大化/关闭），左侧导航 220px，右侧内容区。

~~~
┌──────────┬──────────────────────────────────────┐
│  DLP     │  内容区                               │
│ ○ 首页    │                                       │
│ ○ 版本    │                                       │
│ ○ 实例    │                                       │
│ ○ 下载    │                                       │
│ ○ 设置    │                                       │
│ ○ 日志    │                                       │
│          │                                       │
│ [头像]    │                                       │
└──────────┴──────────────────────────────────────┘
~~~

**页面**
1. **首页**：实例卡片网格（图标、版本、上次游玩时间、启动按钮），右下角账号切换
2. **版本**：已安装版本列表 + 详情（安装状态、Java 版本、删除/修复/打开目录）
3. **下载**：版本清单（正式版/快照/全部筛选、搜索），选中后展示“安装”按钮与进度条
4. **实例**：实例列表、创建、编辑（内存、Java、JVM 参数、分辨率）
5. **设置**：账户、Java 路径、下载源、并发数、语言、主题（深/浅）、便携模式开关
6. **日志**：实时滚动日志 + 复制 / 打开日志目录

**状态机**（首页启动按钮）：
Idle → CheckingFiles → Downloading → Launching → Running → Exited / Failed

**样式**：深色为主、强调色可配（默认紫蓝色 #6C5CE7），圆角 8px，字体 Microsoft YaHei UI。不采用 PCL 的蓝色主色相与界面结构，避免与 PCL 混淆。

---

## 9. 配置

~~~jsonc
// config.json
{
  "language": "zh-CN",
  "theme": "dark",
  "download": { "source": "official", "concurrency": 8, "speedLimitKb": 0 },
  "java": { "autoDetect": true, "defaultPath": null },
  "memoryMb": 2048,
  "portable": false,
  "lastInstance": "Vanilla-26.3"
}
~~~

配置读写：启动时加载，修改即时保存（原子写：临时文件 + File.Replace），损坏时回退默认值并备份。

---

## 10. 错误处理与日志

**错误码**：DLP-1xxx 网络、2xxx 文件/校验、3xxx Java、4xxx 认证、5xxx 启动、9xxx 未知。每条错误含：用户文案、技术详情、建议操作、原始异常（仅日志）。

**日志**：logs/launcher-<日期>.log，格式 [时间] [级别] [模块] 消息；滚动保留 7 天；游戏 stdout/stderr 单独写 logs/game-<版本>-<时间>.log。

**崩溃诊断**：游戏非 0 退出时打包 logs/ + crash-reports/ + hs_err_pid*.log + 版本信息为 logs/crash-<时间>.zip，提示用户“打开诊断包”。不上传。

---

## 11. 安全

- **Zip Slip**：解压前 Path.GetFullPath 校验目标必须在目标目录内；拒绝绝对路径与 ..
- **下载校验**：官方 sha1 必校验；镜像不改变期望 hash；HTTPS 强制
- **Token**：DPAPI 加密；日志打印前脱敏（token/uuid 只显示前 4 位）
- **进程注入**：ArgumentList 传参；不经过 shell
- **路径处理**：所有用户输入路径规范化并校验存在；实例名禁止 ..、/、\、:
- **镜像信任**：镜像仅用于下载，不参与版本 JSON 的信任决策（版本 JSON 仍以官方源 sha1 为准）

---

## 12. 性能目标

| 指标 | 目标 |
|---|---|
| 冷启动到首页可用 | < 2s |
| 版本清单加载（缓存） | < 300ms |
| 安装 1.20 原版（100Mbps） | < 5 分钟 |
| 启动游戏（不含游戏自身加载） | < 1.5s |
| 空闲内存 | < 150MB |

实现手段：清单/资产索引惰性加载；下载并发；UI 线程零 IO；进度批量刷新（100ms 节流）。

---

## 13. 测试策略

- **单元测试**：规则求值（OS/features）、GAV 解析、参数模板替换、路径安全、classpath 顺序、版本继承合并、Java 版本选择
- **集成测试**：本地 HTTP 静态服务器模拟 Mojang，跑完整安装流程；断网/404/坏 hash 场景
- **冒烟测试（真实）**：安装并离线启动 1.12.2（Java 8），验证日志出现 Setting user: 与游戏窗口
- **回归清单**：每次发版手测安装 / 启动 / 实例隔离 / 镜像切换 / 微软登录

---

## 14. 许可与合规

- 全部代码自研，不复制 PCL（Meloong-Git/PCL）及其社区版（PCL-Community/PCL-CE）的受限代码。
- 可参考的只有 Apache-2.0 部分（MeloongCore、PCL.Core），且必须保留其 LICENSE / NOTICE；v1 不直接引入。
- 不实现正版验证绕过；离线模式仅用于离线游戏与开发测试，不接正版服务器。
- 启动器名称、图标、配色独立设计，不使用 PCL 相关名称与蓝色主色相。

---

## 15. 里程碑

| 阶段 | 内容 | 验收 |
|---|---|---|
| **M0** | 设计文档、解决方案骨架、CI 构建 | dotnet build 通过 |
| **M1** | Core：清单 / 下载 / 安装 / Java / 离线启动 + CLI | CLI 能安装并启动 1.12.2 离线 |
| **M2** | WPF UI：首页 / 下载 / 实例 / 设置 / 日志 | 图形界面完成同样流程 |
| **M3** | 微软登录、Fabric/Quilt、整合包导入、自动更新 | 正版账号可登录进服务器 |
| **M4** | 性能优化、崩溃分析、多语言、便携模式 | 达到第 12 章指标 |

---

## 附录 A：启动命令示例（1.12.2 离线）

~~~
java -Xms512M -Xmx2048M
  -Djava.library.path=...\versions\1.12.2\natives
  -Dminecraft.launcher.brand=DLP -Dminecraft.launcher.version=1.0.0
  -cp "...\libraries\...;...\versions\1.12.2\1.12.2.jar"
  net.minecraft.launchwrapper.Launch
  --username Steve --version 1.12.2 --gameDir ...\instances\demo\minecraft
  --assetsDir ...\assets --assetIndex 1.12 --uuid 00000000000000000000000000000000
  --accessToken 0 --userType legacy --versionType release
~~~

## 附录 B：关键外部接口

| 用途 | URL |
|---|---|
| 版本清单 | https://piston-meta.mojang.com/mc/game/version_manifest_v2.json |
| 版本 JSON | manifest 中 url 字段 |
| 资源索引 | 版本 JSON assetIndex.url |
| 资源对象 | https://resources.download.minecraft.net/<前2位>/<hash> |
| 依赖库 | https://libraries.minecraft.net/<maven路径> |
| Mojang JRE 清单 | https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json |
| 微软 OAuth | https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode |
| Xbox Live | https://user.auth.xboxlive.com/user/authenticate |
| XSTS | https://xsts.auth.xboxlive.com/xsts/authorize |
| MC 登录 | https://api.minecraftservices.com/authentication/login_with_xbox |
| MC 档案 | https://api.minecraftservices.com/minecraft/profile |
| Fabric Meta | https://meta.fabricmc.net/v2/versions/loader/<mc> |
| 镜像（备用） | https://bmclapi2.bangbang93.com/ |
