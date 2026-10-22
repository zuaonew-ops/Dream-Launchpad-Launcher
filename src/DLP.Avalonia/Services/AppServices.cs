using System.Net.Http;
using DLP.App.Services;
using DLP.Core.Download;
using DLP.Core.Infrastructure;
using DLP.Core.Install;
using DLP.Core.Java;
using DLP.Core.Launch;
using DLP.Core.Minecraft;

namespace DLP.Avalonia.Services;

/// <summary>应用组合根（Avalonia 版）：与 WPF 版共用 DLP.Common 里的账号/设置/任务服务。</summary>
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
    public ImageCache Images { get; }
    public DownloadMonitor Downloads { get; } = new();

    public event Action? InstalledVersionsChanged;

    public AppServices()
    {
        Paths = GamePaths.CreateDefault();
        Log = new Log(Paths.Logs);
        Http = new HttpClient(new SocketsHttpHandler
        {
            PooledConnectionLifetime = TimeSpan.FromMinutes(5),
            MaxConnectionsPerServer = 64,
            EnableMultipleHttp2Connections = true,
            AutomaticDecompression = System.Net.DecompressionMethods.All,
            ConnectTimeout = TimeSpan.FromSeconds(15)
        })
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
        Images = new ImageCache(Http, Paths, Log);
        Loaders = new LoaderService(Paths, Http, Installer);
        Modpacks = new ModpackInstaller(Paths, Downloader, Installer, Loaders, Modrinth, Log);
    }

    /// <summary>装完版本 / 整合包后通知外壳刷新版本列表。</summary>
    public void NotifyInstalledVersionsChanged() => InstalledVersionsChanged?.Invoke();

    public IReadOnlyList<string> GetInstalledVersions()
    {
        if (!Directory.Exists(Paths.Versions)) return [];
        return Directory.EnumerateDirectories(Paths.Versions)
            .Select(Path.GetFileName)
            .Where(name => name is not null && File.Exists(Path.Combine(Paths.Versions, name!, "install.json")))
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }
}
