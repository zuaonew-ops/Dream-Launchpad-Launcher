using DLP.Core.Auth;
using DLP.Core.Download;
using DLP.Core.Infrastructure;
using DLP.Core.Install;
using DLP.Core.Java;
using DLP.Core.Launch;
using DLP.Core.Minecraft;

var paths = GamePaths.CreateDefault();
using var log = new Log(paths.Logs);
using var http = new HttpClient(new SocketsHttpHandler { PooledConnectionLifetime = TimeSpan.FromMinutes(5) })
{
    Timeout = TimeSpan.FromMinutes(5)
};
http.DefaultRequestHeaders.UserAgent.ParseAdd("DreamLaunchpad/0.1");

var downloader = new DownloadClient(http, log);
var modrinth = new ModrinthService(http);
var manifests = new VersionManifestService(http, paths, log);
var versions = new VersionRepository(paths, downloader, manifests);
var java = new JavaManager(paths, http, downloader, log);
var installer = new GameInstaller(paths, downloader, versions, log);
var launcher = new GameLauncher(paths, log);
Mirrors.Enabled = true;

var command = args.FirstOrDefault() ?? "help";
var rest = args.Skip(1).ToArray();

static string? Option(string[] args, string name)
{
    var index = Array.IndexOf(args, name);
    return index >= 0 && index + 1 < args.Length ? args[index + 1] : null;
}

try
{
    switch (command)
    {
        case "help":
            Console.WriteLine("""
DLP 命令行
  dlp versions [--all] [-n 20]      列出可安装版本
  dlp install <versionId>            安装原版
  dlp java [<versionId>]             扫描/检查 Java（给版本 ID 时确保该版本所需 Java）
  dlp run <versionId> [--username 名字] [--memory 2048] [--java 路径]
  dlp info <versionId>               查看版本详情
  dlp loader <fabric|quilt|forge|neoforge> <mcVersion> [loaderVersion]
                                     列出/安装模组加载器
  dlp modpack <slug> <mcVersion> [--loader fabric]
                                     安装 Modrinth 整合包（.mrpack）
""");
            break;

        case "versions":
        {
            var manifest = await manifests.GetAsync();
            var all = args.Contains("--all");
            var count = int.TryParse(Option(args, "-n"), out var n) ? n : 20;
            var list = all ? manifest.Versions : manifest.Versions.Where(v => v.Type == "release").ToList();
            foreach (var version in list.Take(count))
                Console.WriteLine($"{version.Id,-24} {version.Type,-10} {version.ReleaseTime:yyyy-MM-dd}");
            Console.WriteLine($"最新正式版：{manifest.Latest.Release}；最新快照：{manifest.Latest.Snapshot}");
            break;
        }

        case "loader":
        {
            var typeName = rest.FirstOrDefault() ?? throw new ArgumentException("用法：dlp loader <fabric|quilt|forge|neoforge> <mcVersion> [loaderVersion]");
            var mcVersion = rest.Skip(1).FirstOrDefault() ?? throw new ArgumentException("缺少原版版本 ID");
            var loaderType = typeName.ToLowerInvariant() switch
            {
                "fabric" => ModLoaderType.Fabric,
                "quilt" => ModLoaderType.Quilt,
                "forge" => ModLoaderType.Forge,
                "neoforge" => ModLoaderType.NeoForge,
                _ => throw new ArgumentException($"未知加载器：{typeName}")
            };

            var loaders = new LoaderService(paths, http, installer);
            if (rest.Length < 3)
            {
                var list = await loaders.ListAsync(loaderType, mcVersion, CancellationToken.None);
                foreach (var item in list.Take(10))
                    Console.WriteLine($"{item.Id,-32} {item.DisplayName}");
                Console.WriteLine($"共 {list.Count} 个 {typeName} 构建，安装：dlp loader {typeName} {mcVersion} <loaderVersion>");
                break;
            }

            var installed = await loaders.InstallAsync(loaderType, mcVersion, rest[2],
                new Progress<string>(Console.WriteLine), CancellationToken.None);
            Console.WriteLine($"安装完成：{installed}");
            break;
        }

        case "modpack":
        {
            var slug = rest.FirstOrDefault() ?? throw new ArgumentException("缺少整合包 slug");
            var mc = rest.Skip(1).FirstOrDefault() ?? throw new ArgumentException("缺少 Minecraft 版本");
            var loader = Option(args, "--loader");
            var modpacker = new ModpackInstaller(paths, downloader, installer,
                new LoaderService(paths, http, installer), modrinth, log);
            var result = await modpacker.InstallAsync(slug, mc, loader, null, CancellationToken.None);
            Console.WriteLine($"安装完成：{result.VersionId}（{result.FilesInstalled} 个文件，{result.OverridesInstalled} 个覆盖文件）");
            Console.WriteLine($"游戏目录：{result.GameDir}");
            return 0;
        }

        case "info":
        {
            var id = rest.FirstOrDefault() ?? throw new ArgumentException("缺少版本 ID");
            var version = await versions.ResolveAsync(id);
            Console.WriteLine($"ID: {version.Id}");
            Console.WriteLine($"主类: {version.MainClass}");
            Console.WriteLine($"资源索引: {version.AssetIndex?.Id} ({version.AssetIndex?.TotalSize / 1024 / 1024} MB)");
            Console.WriteLine($"依赖库: {version.Libraries.Count} 个；Java 要求: {version.JavaVersion?.MajorVersion} ({version.JavaVersion?.Component})");
            break;
        }

        case "install":
        {
            var id = rest.FirstOrDefault() ?? throw new ArgumentException("缺少版本 ID");
            await installer.InstallAsync(id);
            var version = await versions.ResolveAsync(id);
            var runtime = await java.EnsureAsync(version);
            Console.WriteLine($"安装完成：{id}，Java {runtime.MajorVersion}（{runtime.Path}）");
            break;
        }

        case "java":
        {
            var id = rest.FirstOrDefault();
            if (id is null)
            {
                var runtimes = java.Scan();
                foreach (var runtime in runtimes)
                    Console.WriteLine($"Java {runtime.MajorVersion,-3} {(runtime.Is64Bit ? "64位" : "32位")} {runtime.RawVersion,-16} {runtime.Path}");
                if (runtimes.Count == 0) Console.WriteLine("未发现任何 Java，可用 dlp install <版本> 自动下载。");
            }
            else
            {
                var version = await versions.ResolveAsync(id);
                var runtime = await java.EnsureAsync(version);
                Console.WriteLine($"使用 Java {runtime.MajorVersion}：{runtime.Path}");
            }
            break;
        }

        case "run":
        {
            var id = rest.FirstOrDefault() ?? throw new ArgumentException("缺少版本 ID");
            var username = Option(args, "--username") ?? "Steve";
            var memory = int.TryParse(Option(args, "--memory"), out var m) ? m : 2048;
                        var version = await versions.ResolveAsync(id);

            // 加载器版本没有自己的 client jar，只要有 json 且能继承父版本即可启动。
            var runnable = File.Exists(paths.VersionJson(id))
                           && (File.Exists(paths.VersionJar(id)) || version.InheritsFrom is not null);
            if (!runnable)
            {
                var installId = version.InheritsFrom ?? id;
                log.Info($"未检测到安装，先安装 {installId}");
                await installer.InstallAsync(installId);
            }

            var runtime = Option(args, "--java") is { } javaPath
                ? new JavaRuntime(javaPath, 0, true, "manual", javaPath)
                : await java.EnsureAsync(version);

            var gameDir = paths.GameDir(id);
            Directory.CreateDirectory(gameDir);
            var options = new LaunchOptions
            {
                Account = LaunchAccount.Offline(username),
                JavaPath = runtime.Path,
                GameDirectory = gameDir,
                MemoryMb = memory
            };

            var process = launcher.BuildProcess(version, options);
            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
            log.Info($"游戏已启动，PID {process.Id}");
            await process.WaitForExitAsync();
            Console.WriteLine($"游戏退出，退出码 {process.ExitCode}");
            return process.ExitCode;
        }

        default:
            Console.WriteLine($"未知命令：{command}（dlp help 查看用法）");
            return 1;
    }
}
catch (Exception ex)
{
    log.Error(ex.ToString());
    Console.WriteLine($"失败：{ex.Message}");
    return 1;
}
return 0;