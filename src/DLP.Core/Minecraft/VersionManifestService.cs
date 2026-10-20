using System.Net;
using System.Text.Json;
using DLP.Core.Download;
using DLP.Core.Infrastructure;

namespace DLP.Core.Minecraft;

/// <summary>版本清单：带 ETag 的本地缓存，失败时回退镜像。</summary>
public sealed class VersionManifestService(HttpClient http, GamePaths paths, Log log)
{
    private const string OfficialUrl = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    private const string MirrorUrl = "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

    private readonly string _cacheFile = Path.Combine(paths.Cache, "version_manifest_v2.json");
    private readonly string _etagFile = Path.Combine(paths.Cache, "version_manifest_v2.etag");
    private VersionManifest? _cached;

    public async Task<VersionManifest> GetAsync(bool force = false, CancellationToken ct = default)
    {
        if (!force && _cached is not null) return _cached;

        if (!force && File.Exists(_cacheFile) && File.GetLastWriteTimeUtc(_cacheFile) > DateTime.UtcNow.AddHours(-1))
        {
            try { return _cached = ReadCache(); } catch { /* 缓存损坏则重新下载 */ }
        }

        var etag = File.Exists(_etagFile) ? File.ReadAllText(_etagFile).Trim() : null;
        var request = new HttpRequestMessage(HttpMethod.Get, Mirrors.Rewrite(OfficialUrl));
        if (etag is not null) request.Headers.TryAddWithoutValidation("If-None-Match", etag);

        try
        {
            using var response = await http.SendAsync(request, ct);
            if (response.StatusCode == HttpStatusCode.NotModified && File.Exists(_cacheFile))
                return _cached = ReadCache();
            response.EnsureSuccessStatusCode();
            var json = await response.Content.ReadAsStringAsync(ct);
            await File.WriteAllTextAsync(_cacheFile, json, ct);
            if (response.Headers.ETag?.Tag is { } tag) await File.WriteAllTextAsync(_etagFile, tag, ct);
            _cached = JsonSerializer.Deserialize<VersionManifest>(json) ?? throw new VersionException("版本清单为空");
            return _cached;
        }
        catch (Exception ex)
        {
            log.Warn($"官方版本清单获取失败，尝试镜像：{ex.Message}");
            var json = await http.GetStringAsync(MirrorUrl, ct);
            await File.WriteAllTextAsync(_cacheFile, json, ct);
            _cached = JsonSerializer.Deserialize<VersionManifest>(json) ?? throw new VersionException("镜像版本清单为空");
            return _cached;
        }
    }

    private VersionManifest ReadCache()
    {
        var json = File.ReadAllText(_cacheFile);
        return JsonSerializer.Deserialize<VersionManifest>(json) ?? throw new VersionException("版本清单缓存无效");
    }
}

public sealed class VersionException(string message, Exception? inner = null) : Exception(message, inner);
