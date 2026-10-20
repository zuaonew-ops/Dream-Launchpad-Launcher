using System.IO.Compression;
using System.Text.Json;
using DLP.Core.Download;
using DLP.Core.Infrastructure;
using DLP.Core.Minecraft;

namespace DLP.Core.Install;

public sealed record InstallProgress(string Stage, int Completed, int Total, long DownloadedBytes, long TotalBytes);

/// <summary>原版版本安装：client jar、依赖库、natives、资源、日志配置。</summary>
public sealed class GameInstaller(GamePaths paths, DownloadClient downloader,
    VersionRepository versions, Log log)
{
    public async Task InstallAsync(string versionId, IProgress<InstallProgress>? progress = null, CancellationToken ct = default)
    {
        log.Info($"开始安装 {versionId}");
        var version = await versions.ResolveAsync(versionId, ct);

        var jar = version.Downloads.Client ?? throw new VersionException($"{versionId} 缺少 client 下载信息");
        progress?.Report(new InstallProgress("client", 0, 1, 0, jar.Size));
        await downloader.DownloadAsync(jar.Url, paths.VersionJar(versionId), jar.Sha1, jar.Size, ct);

        await InstallLibrariesAsync(version, ct);
        await InstallAssetsAsync(version, progress, ct);
        await InstallLoggingAsync(version, ct);

        await File.WriteAllTextAsync(Path.Combine(paths.VersionDir(versionId), "install.json"),
            JsonSerializer.Serialize(new { version = versionId, installedAt = DateTimeOffset.Now }, new JsonSerializerOptions { WriteIndented = true }), ct);
        log.Info($"安装完成 {versionId}");
    }

    private async Task InstallLibrariesAsync(VersionJson version, CancellationToken ct)
    {
        var nativesDir = paths.NativesDir(version.Id);
        Directory.CreateDirectory(nativesDir);
        var jobs = new List<Task>();
        using var gate = new SemaphoreSlim(8);

        foreach (var library in version.Libraries)
        {
            if (!RuleEvaluator.IsAllowed(library.Rules, OsContext.Windows)) continue;

            if (library.Natives is not null)
            {
                if (!library.Natives.TryGetValue("windows", out var classifierKey)) continue;
                var classifier = classifierKey.Replace("${arch}", "64");
                if (library.Downloads?.Classifiers is not { } classifiers ||
                    !classifiers.TryGetValue(classifier, out var nativeArtifact)) continue;

                var target = Path.Combine(paths.Libraries, nativeArtifact.Path ?? MavenCoordinate.Parse(library.Name).ToPath());
                await gate.WaitAsync(ct);
                jobs.Add(Task.Run(async () =>
                {
                    try
                    {
                        await downloader.DownloadAsync(nativeArtifact.Url, target, nativeArtifact.Sha1, nativeArtifact.Size, ct);
                        ExtractNatives(target, nativesDir, library.Extract);
                    }
                    finally { gate.Release(); }
                }, ct));
                continue;
            }

            if (library.Downloads?.Artifact is not { } artifact) continue;
            var libPath = Path.Combine(paths.Libraries, artifact.Path ?? MavenCoordinate.Parse(library.Name).ToPath());
            await gate.WaitAsync(ct);
            jobs.Add(Task.Run(async () =>
            {
                try { await downloader.DownloadAsync(artifact.Url, libPath, artifact.Sha1, artifact.Size, ct); }
                finally { gate.Release(); }
            }, ct));
        }

        await Task.WhenAll(jobs);
    }

    private static void ExtractNatives(string jarPath, string nativesDir, ExtractRule? extract)
    {
        var excludes = extract?.Exclude ?? [];
        using var zip = ZipFile.OpenRead(jarPath);
        foreach (var entry in zip.Entries)
        {
            if (entry.FullName.EndsWith('/')) continue;
            if (excludes.Any(e => entry.FullName.StartsWith(e, StringComparison.OrdinalIgnoreCase))) continue;

            var destination = Path.GetFullPath(Path.Combine(nativesDir, entry.FullName));
            if (!destination.StartsWith(Path.GetFullPath(nativesDir) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                continue; // Zip Slip 防护

            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
            entry.ExtractToFile(destination, overwrite: true);
        }
    }

    private async Task InstallAssetsAsync(VersionJson version, IProgress<InstallProgress>? progress, CancellationToken ct)
    {
        if (version.AssetIndex is not { } indexRef)
        {
            log.Warn("版本没有 assetIndex，跳过资源下载");
            return;
        }

        var indexPath = Path.Combine(paths.AssetIndexes, indexRef.Id + ".json");
        await downloader.DownloadAsync(indexRef.Url, indexPath, indexRef.Sha1, indexRef.Size, ct);

        var index = JsonSerializer.Deserialize<AssetIndex>(await File.ReadAllTextAsync(indexPath, ct))
                    ?? throw new VersionException("资源索引无效");

        var unique = index.Objects.Values.GroupBy(o => o.Hash).Select(g => g.First()).ToList();
        var done = 0;
        using var gate = new SemaphoreSlim(16);
        var jobs = new List<Task>(unique.Count);

        foreach (var asset in unique)
        {
            var relative = Path.Combine(asset.Hash[..2], asset.Hash);
            var target = Path.Combine(paths.AssetObjects, relative);
            await gate.WaitAsync(ct);
            jobs.Add(Task.Run(async () =>
            {
                try
                {
                    var url = $"https://resources.download.minecraft.net/{asset.Hash[..2]}/{asset.Hash}";
                    await downloader.DownloadAsync(url, target, asset.Hash, asset.Size, ct);
                    var current = Interlocked.Increment(ref done);
                    if (current % 200 == 0) progress?.Report(new InstallProgress("assets", current, unique.Count, 0, indexRef.TotalSize));
                }
                finally { gate.Release(); }
            }, ct));
        }

        await Task.WhenAll(jobs);
        progress?.Report(new InstallProgress("assets", unique.Count, unique.Count, 0, indexRef.TotalSize));
    }

    private async Task InstallLoggingAsync(VersionJson version, CancellationToken ct)
    {
        if (version.Logging?.Client?.File is not { } file || string.IsNullOrEmpty(file.Url)) return;
        var target = Path.Combine(paths.VersionDir(version.Id), "log4j.xml");
        await downloader.DownloadAsync(file.Url, target, file.Sha1, file.Size, ct);
    }
}