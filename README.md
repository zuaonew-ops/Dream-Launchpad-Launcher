# Dream Launchpad Launcher

从零设计的 Minecraft: Java Edition 启动器（代号 **DLP**）。

## 当前进度

- [x] 设计文档
- [x] 核心库：清单、下载、安装、Java、离线启动
- [x] 命令行（DLP.Cli）
- [x] 界面：首页 / 版本 / 实例 / 设置 / 日志
- [ ] 微软登录、Fabric/Quilt、整合包导入

## 界面预览

![首页](docs/screenshots/home.png)
![版本库](docs/screenshots/versions.png)

浅色界面，左侧导航，内容区卡片式布局：

- **首页**：账号、启动版本、内存上限、启动按钮与状态；实例、数据目录、统计
- **版本**：搜索 / 筛选版本，安装进度条，已安装标记
- **实例**：实例列表、创建实例、打开目录；每个实例独立 mods / saves / 配置
- **设置**：默认内存、用户名、Java 路径、下载源、Java 扫描
- **日志**：启动器与游戏输出实时滚动

## 构建与运行

需要 .NET 10 SDK（界面）或 .NET 10 Desktop Runtime（仅运行）。

~~~bash
dotnet build
dotnet run --project src/DLP.App          # 启动界面
dotnet publish src/DLP.App -c Release -o dist/DLP.App
~~~

命令行：

~~~bash
dotnet run --project src/DLP.Cli -- versions
dotnet run --project src/DLP.Cli -- install 1.12.2
dotnet run --project src/DLP.Cli -- run 1.12.2 --username Steve --memory 2048
~~~

## 结构

~~~
DreamLaunchpad.slnx
├─ src/DLP.Core    核心库（清单、下载、安装、Java、认证、启动）
├─ src/DLP.Cli     命令行
├─ src/DLP.App     界面
└─ tests/DLP.Core.Tests
~~~

## 数据目录

默认 %APPDATA%\DreamLaunchpad；exe 同级放 portable.txt 切换便携模式。

## 文档

- [设计文档](docs/设计-Dream-Launchpad-Launcher.md)
- [PCL / PCL-CE 参考调研](docs/PCL-参考仓库调研.md)
