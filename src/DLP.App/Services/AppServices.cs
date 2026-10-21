using System.IO;
using System.Net.Http;
using DLP.Core.Download;
using DLP.Core.Infrastructure;
using DLP.Core.Install;
using DLP.Core.Java;
using DLP.Core.Launch;
using DLP.Core.Minecraft;

namespace DLP.App.Services;

/// <summary>应用组合根：所有核心服务在此创建一次，供各 ViewModel 共用。</summary>
public sealed class AppServices
{
    public GamePaths Paths { get; }
    public Log Log { get; }
    public HttpClient Http { get; }
    public DownloadClient Downloader { get; }
    public VersionManifestService Manifests { get; }
    public VersionRepository Versions { get; }
    public JavaManager Java { get; }
    public GameInstaller Installer { get; }
    public GameLauncher Launcher { get; }
    public AppSettings Settings { get; }
    public AccountManager Accounts { get; }
    public ModrinthService Modrinth { get; }
    public LoaderService Loaders { get; }
    /// <summary>下载用 HTTP 客户端：多连接、HTTP/2、连接复用，避免并发下载时被连接数卡住。</summary>
    internal static SocketsHttpHandler CreateHandler() => new()
    {
        PooledConnectionLifetime = TimeSpan.FromMinutes(5),
        MaxConnectionsPerServer = 64,
        EnableMultipleHttp2Connections = true,
        AutomaticDecompression = System.Net.DecompressionMethods.All,
        ConnectTimeout = TimeSpan.FromSeconds(15)
    };

    public ModpackInstaller Modpacks { get; }
    public DownloadMonitor Downloads { get; } = new();

    public event Action? InstalledVersionsChanged;

    public AppServices()
    {
        Paths = GamePaths.CreateDefault();
        Log = new Log(Paths.Logs);
        Http = new HttpClient(CreateHandler())
        {
            Timeout = TimeSpan.FromMinutes(10)
        };
        Http.DefaultRequestHeaders.UserAgent.ParseAdd("DreamLaunchpad/0.3");
        Downloader = new DownloadClient(Http, Log);
        Manifests = new VersionManifestService(Http, Paths, Log);
        Versions = new VersionRepository(Paths, Downloader, Manifests);
        Java = new JavaManager(Paths, Http, Downloader, Log);
        Installer = new GameInstaller(Paths, Downloader, Versions, Log);
        Launcher = new GameLauncher(Paths, Log);
        Settings = AppSettings.Load();
        Mirrors.Enabled = Settings.DownloadSource != "official";
        Accounts = new AccountManager(Paths, Settings, Http);
        Modrinth = new ModrinthService(Http);
        Loaders = new LoaderService(Paths, Http, Installer);
        Modpacks = new ModpackInstaller(Paths, Downloader, Installer, Loaders, Modrinth, Log);
    }

    /// <summary>版本列表（已过滤掉作为依赖装进来的原版，例如装 Forge 时顺带装的那个原版）。</summary>
    public IReadOnlyList<string> GetInstalledVersions()
    {
        var all = GetAllInstalledVersions();
        if (all.Count == 0) return all;

        var hidden = FindDependencyVersions(all);
        return all.Where(id => !hidden.Contains(id)).ToList();
    }

    /// <summary>磁盘上所有可启动版本，包含被隐藏的依赖版本。</summary>
    public IReadOnlyList<string> GetAllInstalledVersions()
    {
        if (!Directory.Exists(Paths.Versions)) return [];
        return Directory.EnumerateDirectories(Paths.Versions)
            .Select(Path.GetFileName)
            .Where(name => name is not null && IsRunnableVersion(name))
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    /// <summary>找出只作为加载器/整合包依赖存在的原版版本。</summary>
    private HashSet<string> FindDependencyVersions(IReadOnlyList<string> all)
    {
        var hidden = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var children = new Dictionary<string, List<string>>(StringComparer.OrdinalIgnoreCase);

        foreach (var id in all)
        {
            var role = ReadInstallRole(id);
            if (role == "dependency") hidden.Add(id);

            if (ReadInheritsFrom(id) is { } parent)
            {
                if (!children.TryGetValue(parent, out var list)) children[parent] = list = [];
                list.Add(id);
            }
        }

        // 兼容旧安装：以前装加载器时父版本没有 role 标记，只要下面挂了加载器版本就当作依赖隐藏。
        foreach (var id in all)
        {
            if (hidden.Contains(id) || ReadInstallRole(id) is not null) continue;
            if (children.TryGetValue(id, out var kids) && kids.Any(LooksLikeLoaderVersion)) hidden.Add(id);
        }

        return hidden;
    }

    /// <summary>加载器安装出来的版本 id 形如 1.12.2-forge-…、fabric-loader-0.15.0-1.20.1。</summary>
    private static bool LooksLikeLoaderVersion(string id)
        => id.Contains("-forge-", StringComparison.OrdinalIgnoreCase)
           || id.Contains("-neoforge-", StringComparison.OrdinalIgnoreCase)
           || id.StartsWith("fabric-loader-", StringComparison.OrdinalIgnoreCase)
           || id.StartsWith("quilt-loader-", StringComparison.OrdinalIgnoreCase)
           || id.StartsWith("forge-", StringComparison.OrdinalIgnoreCase)
           || id.StartsWith("neoforge-", StringComparison.OrdinalIgnoreCase);

    private string? ReadInstallRole(string id)
    {
        var path = Path.Combine(Paths.VersionDir(id), "install.json");
        if (!File.Exists(path)) return null;
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(File.ReadAllText(path));
            return doc.RootElement.TryGetProperty("role", out var role) && role.ValueKind == System.Text.Json.JsonValueKind.String
                ? role.GetString()
                : null;
        }
        catch { return null; }
    }

    private string? ReadInheritsFrom(string id)
    {
        var path = Paths.VersionJson(id);
        if (!File.Exists(path)) return null;
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(File.ReadAllText(path));
            return doc.RootElement.TryGetProperty("inheritsFrom", out var value) && value.ValueKind == System.Text.Json.JsonValueKind.String
                ? value.GetString()
                : null;
        }
        catch { return null; }
    }

    /// <summary>删掉加载器版本后，把没人再用的依赖原版一并清理，避免残留占空间。</summary>
    public int CleanupOrphanDependencies()
    {
        var all = GetAllInstalledVersions();
        var hidden = FindDependencyVersions(all);
        var parentsInUse = all
            .Select(ReadInheritsFrom)
            .Where(parent => parent is not null)
            .Select(parent => parent!)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        var removed = 0;
        foreach (var id in hidden)
        {
            if (parentsInUse.Contains(id)) continue;
            try
            {
                var dir = Paths.VersionDir(id);
                if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
                removed++;
            }
            catch { /* 占用中就先留着 */ }
        }
        return removed;
    }

    /// <summary>有 json 且（自带 client jar 或通过 inheritsFrom 继承父版本的 jar）才算可启动。</summary>
    private bool IsRunnableVersion(string id)
    {
        var json = Paths.VersionJson(id);
        if (!File.Exists(json)) return false;
        if (File.Exists(Paths.VersionJar(id))) return true;
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(File.ReadAllText(json));
            return doc.RootElement.TryGetProperty("inheritsFrom", out var value)
                   && value.ValueKind == System.Text.Json.JsonValueKind.String
                   && !string.IsNullOrWhiteSpace(value.GetString());
        }
        catch
        {
            return false;
        }
    }

    public void NotifyInstalledVersionsChanged() => InstalledVersionsChanged?.Invoke();
}
