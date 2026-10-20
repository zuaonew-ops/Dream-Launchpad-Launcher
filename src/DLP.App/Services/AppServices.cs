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

    public event Action? InstalledVersionsChanged;

    public AppServices()
    {
        Paths = GamePaths.CreateDefault();
        Log = new Log(Paths.Logs);
        Http = new HttpClient(new SocketsHttpHandler { PooledConnectionLifetime = TimeSpan.FromMinutes(5) })
        {
            Timeout = TimeSpan.FromMinutes(10)
        };
        Http.DefaultRequestHeaders.UserAgent.ParseAdd("DreamLaunchpad/0.1");
        Downloader = new DownloadClient(Http, Log);
        Manifests = new VersionManifestService(Http, Paths, Log);
        Versions = new VersionRepository(Paths, Downloader, Manifests);
        Java = new JavaManager(Paths, Http, Downloader, Log);
        Installer = new GameInstaller(Paths, Downloader, Versions, Log);
        Launcher = new GameLauncher(Paths, Log);
        Settings = AppSettings.Load();
    }

    public IReadOnlyList<string> GetInstalledVersions()
    {
        if (!Directory.Exists(Paths.Versions)) return [];
        return Directory.EnumerateDirectories(Paths.Versions)
            .Select(Path.GetFileName)
            .Where(name => name is not null
                           && File.Exists(Paths.VersionJson(name))
                           && File.Exists(Paths.VersionJar(name)))
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    public IReadOnlyList<string> GetInstances()
    {
        if (!Directory.Exists(Paths.Instances)) return [];
        return Directory.EnumerateDirectories(Paths.Instances)
            .Select(Path.GetFileName)
            .Where(name => name is not null)
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    public void NotifyInstalledVersionsChanged() => InstalledVersionsChanged?.Invoke();
}