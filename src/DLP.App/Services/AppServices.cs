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
    public ModpackInstaller Modpacks { get; }

    public event Action? InstalledVersionsChanged;

    public AppServices()
    {
        Paths = GamePaths.CreateDefault();
        Log = new Log(Paths.Logs);
        Http = new HttpClient(new SocketsHttpHandler { PooledConnectionLifetime = TimeSpan.FromMinutes(5) })
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

    public IReadOnlyList<string> GetInstalledVersions()
    {
        if (!Directory.Exists(Paths.Versions)) return [];
        return Directory.EnumerateDirectories(Paths.Versions)
            .Select(Path.GetFileName)
            .Where(name => name is not null && IsRunnableVersion(name))
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();
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
