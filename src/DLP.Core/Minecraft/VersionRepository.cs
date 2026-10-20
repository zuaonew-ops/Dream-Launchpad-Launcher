using System.Text.Json;
using DLP.Core.Download;
using DLP.Core.Infrastructure;

namespace DLP.Core.Minecraft;

/// <summary>版本 JSON 的获取与 inheritsFrom 合并。</summary>
public sealed class VersionRepository(GamePaths paths, DownloadClient downloader, VersionManifestService manifests)
{
    public async Task<VersionJson> GetRawAsync(string id, CancellationToken ct = default)
    {
        var local = paths.VersionJson(id);
        if (File.Exists(local))
            return JsonSerializer.Deserialize<VersionJson>(await File.ReadAllTextAsync(local, ct))
                   ?? throw new VersionException($"版本 JSON 损坏：{id}");

        var manifest = await manifests.GetAsync(ct: ct);
        var entry = manifest.Versions.FirstOrDefault(v => v.Id == id)
                    ?? throw new VersionException($"版本不存在：{id}");

        Directory.CreateDirectory(paths.VersionDir(id));
        await downloader.DownloadAsync(entry.Url, local, entry.Sha1, ct: ct);
        return JsonSerializer.Deserialize<VersionJson>(await File.ReadAllTextAsync(local, ct))
               ?? throw new VersionException($"版本 JSON 无效：{id}");
    }

    public async Task<VersionJson> ResolveAsync(string id, CancellationToken ct = default)
    {
        var child = await GetRawAsync(id, ct);
        if (child.InheritsFrom is not { } parentId) return child;

        var parent = await ResolveAsync(parentId, ct);
        return Merge(parent, child);
    }

    private static VersionJson Merge(VersionJson parent, VersionJson child)
    {
        var merged = new VersionJson
        {
            Id = child.Id,
            InheritsFrom = child.InheritsFrom,
            Type = child.Type,
            MainClass = string.IsNullOrEmpty(child.MainClass) ? parent.MainClass : child.MainClass,
            Assets = child.Assets ?? parent.Assets,
            AssetIndex = child.AssetIndex ?? parent.AssetIndex,
            Downloads = new VersionDownloads
            {
                Client = child.Downloads.Client ?? parent.Downloads.Client,
                Server = child.Downloads.Server ?? parent.Downloads.Server
            },
            Libraries = [.. parent.Libraries, .. child.Libraries],
            Arguments = new VersionArguments
            {
                Game = [.. parent.Arguments?.Game ?? [], .. child.Arguments?.Game ?? []],
                Jvm = [.. parent.Arguments?.Jvm ?? [], .. child.Arguments?.Jvm ?? []]
            },
            MinecraftArguments = child.MinecraftArguments ?? parent.MinecraftArguments,
            JavaVersion = child.JavaVersion ?? parent.JavaVersion,
            Logging = child.Logging ?? parent.Logging,
            MinimumLauncherVersion = child.MinimumLauncherVersion ?? parent.MinimumLauncherVersion,
            ReleaseTime = child.ReleaseTime ?? parent.ReleaseTime,
            Time = child.Time ?? parent.Time
        };
        return merged;
    }
}