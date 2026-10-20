# Dream Launchpad Launcher

从零设计的 Minecraft: Java Edition 启动器（代号 **DLP**）。

## 当前进度

- [x] 设计文档
- [x] 解决方案骨架（DLP.Core / DLP.Cli / DLP.App / 测试）
- [x] 版本清单获取与缓存
- [x] 原版安装：client jar、依赖库、natives、资源、日志配置
- [x] Java 扫描 / 选择 / Mojang 运行时下载
- [x] 离线账号（UUID v3）与启动参数构建
- [ ] WPF 界面（进行中）
- [ ] 微软登录、Fabric/Quilt、整合包导入

## 结构

~~~
DreamLaunchpad.slnx
├─ src/DLP.Core    核心库（清单、下载、安装、Java、认证、启动）
├─ src/DLP.Cli     命令行（调试与自动化）
├─ src/DLP.App     WPF 界面
└─ tests/DLP.Core.Tests
~~~

## 命令行

~~~bash
dotnet run --project src/DLP.Cli -- versions          # 列出版本
dotnet run --project src/DLP.Cli -- install 1.12.2    # 安装
dotnet run --project src/DLP.Cli -- run 1.12.2 --username Steve --memory 2048
~~~

## 数据目录

默认 %APPDATA%\DreamLaunchpad；exe 同级放 portable.txt 切换为便携模式。

## 文档

- [设计文档](docs/设计-Dream-Launchpad-Launcher.md)
- [PCL / PCL-CE 参考调研](docs/PCL-参考仓库调研.md)