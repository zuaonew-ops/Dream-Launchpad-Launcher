using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace DLP.Core.Download;

/// <summary>Modrinth 内容类型。</summary>
public enum ContentKind
{
    Mod,
    Modpack,
    Shader,
    ResourcePack,
    DataPack,
}

/// <summary>搜索结果中的一个项目。</summary>
public sealed record ContentProject(
    string Id,
    string Slug,
    string Title,
    string Description,
    string? IconUrl,
    long Downloads,
    string Author);

/// <summary>某个项目的一个版本，下载信息取自主文件（files[].primary，无则第一个）。</summary>
public sealed record ContentVersion(
    string Id,
    string Name,
    string VersionNumber,
    string GameVersions,
    string Loaders,
    string? DownloadUrl,
    string? FileName,
    long FileSize,
    string? Sha1);

/// <summary>
/// Modrinth 内容浏览与下载（API 文档：https://docs.modrinth.com/api/）。
/// 本类不引用 DLP.Core 的其他类型：搜索、版本解析、文件下载（重试 / SHA1 校验 / .part 临时文件）都在此实现。
/// </summary>
public sealed class ModrinthService(HttpClient http)
{
    private const string ApiBase = "https://api.modrinth.com/v2";
    private const string UserAgent = "DreamLaunchpad/0.3 (local launcher)";
    private const int MaxAttempts = 3;
    private const int BufferSize = 81920;

    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };

    /// <summary>搜索项目。facets 用 JSON 数组字符串（如 [["project_type:mod"],["versions:1.20.1"]]）并做 URL 编码。</summary>
    public async Task<IReadOnlyList<ContentProject>> SearchAsync(
        ContentKind kind,
        string query,
        string? gameVersion,
        string? loader,
        int limit,
        CancellationToken ct,
        string sortIndex = "relevance")
    {
        var effectiveLimit = Math.Clamp(limit, 1, 100);

        var facets = new List<string[]> { new[] { $"project_type:{ProjectType(kind)}" } };
        if (!string.IsNullOrWhiteSpace(gameVersion)) facets.Add(new[] { $"versions:{gameVersion.Trim()}" });
        if (!string.IsNullOrWhiteSpace(loader)) facets.Add(new[] { $"categories:{loader.Trim()}" });

        var facetsJson = JsonSerializer.Serialize(facets);
        var url = $"{ApiBase}/search?query={Uri.EscapeDataString(query ?? string.Empty)}" +
                  $"&index={Uri.EscapeDataString(sortIndex)}" +
                  $"&limit={effectiveLimit}" +
                  $"&facets={Uri.EscapeDataString(facetsJson)}";

        var json = await GetJsonAsync(url, ct);
        var response = Deserialize<SearchResponse>(json, "搜索");

        return (response.Hits ?? new List<SearchHit>())
            .Select(h => new ContentProject(
                Id: h.ProjectId ?? h.Slug ?? string.Empty,
                Slug: h.Slug ?? string.Empty,
                Title: h.Title ?? string.Empty,
                Description: h.Description ?? string.Empty,
                IconUrl: string.IsNullOrWhiteSpace(h.IconUrl) ? null : h.IconUrl,
                Downloads: h.Downloads,
                Author: h.Author ?? string.Empty))
            .ToList();
    }

    /// <summary>按下载量取热门内容（打开分类时默认展示，不需要用户先输入关键词）。</summary>
    public Task<IReadOnlyList<ContentProject>> FeaturedAsync(
        ContentKind kind,
        string? gameVersion,
        string? loader,
        int limit,
        CancellationToken ct)
        => SearchAsync(kind, string.Empty, gameVersion, loader, limit, ct, sortIndex: "downloads");

    /// <summary>获取项目的版本列表（按 Modrinth 返回顺序，最新在前）。</summary>
    public async Task<IReadOnlyList<ContentVersion>> GetVersionsAsync(
        ContentKind kind,
        string projectId,
        string? gameVersion,
        string? loader,
        CancellationToken ct)
    {
        var versions = await FetchVersionsAsync(kind, projectId, gameVersion, loader, ct);
        return versions.Select(ToContentVersion).ToList();
    }

    /// <summary>
    /// 安装（下载）项目最新匹配版本的主文件，返回最终文件的绝对路径。
    /// 目录约定：Mod -> mods，Shader -> shaderpacks，ResourcePack -> resourcepacks，DataPack -> datapacks，Modpack -> modpacks。
    /// Modpack 只下载 .mrpack 整合包文件本身，不解析其内部的 mod 清单（整合包安装由调用方另行处理）。
    /// </summary>
    public async Task<string> InstallAsync(
        ContentKind kind,
        string projectId,
        string gameVersion,
        string? loader,
        string gameRoot,
        IProgress<double>? progress,
        CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(projectId);
        ArgumentException.ThrowIfNullOrWhiteSpace(gameVersion);
        ArgumentException.ThrowIfNullOrWhiteSpace(gameRoot);

        var versions = await FetchVersionsAsync(kind, projectId, gameVersion, loader, ct);
        if (versions.Count == 0)
        {
            var loaderText = string.IsNullOrWhiteSpace(loader) ? string.Empty : $"，加载器 {loader}";
            throw new InvalidOperationException($"Modrinth 上未找到匹配的版本：{projectId}（游戏版本 {gameVersion}{loaderText}）");
        }

        var latest = versions[0];
        var file = SelectDownloadFile(kind, latest.Files)
                   ?? throw new InvalidOperationException($"版本 {latest.VersionNumber ?? "未知"} 没有可下载的文件：{projectId}");

        var url = file.Url!;
        var fileName = SafeFileName(file.Filename, url);
        var destination = Path.GetFullPath(Path.Combine(Path.GetFullPath(gameRoot), TargetFolder(kind), fileName));

        return await DownloadFileAsync(url, destination, file.Size, file.Hashes?.Sha1, progress, ct);
    }

    // ---------- 版本 / 文件选择 ----------

    private async Task<IReadOnlyList<VersionDto>> FetchVersionsAsync(
        ContentKind kind,
        string projectId,
        string? gameVersion,
        string? loader,
        CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(projectId);

        // 版本接口按项目 id 查询，本身不需要 project_type；这里仅校验枚举值合法。
        _ = ProjectType(kind);

        var query = new List<string>();
        if (!string.IsNullOrWhiteSpace(gameVersion))
            query.Add($"game_versions={Uri.EscapeDataString(JsonSerializer.Serialize(new[] { gameVersion.Trim() }))}");
        if (!string.IsNullOrWhiteSpace(loader))
            query.Add($"loaders={Uri.EscapeDataString(JsonSerializer.Serialize(new[] { loader.Trim() }))}");

        var url = $"{ApiBase}/project/{Uri.EscapeDataString(projectId)}/version";
        if (query.Count > 0) url += "?" + string.Join("&", query);

        var json = await GetJsonAsync(url, ct);
        return Deserialize<List<VersionDto>>(json, "版本列表");
    }

    private static ContentVersion ToContentVersion(VersionDto version)
    {
        var files = version.Files ?? new List<VersionFileDto>();
        var file = files.FirstOrDefault(f => f.Primary) ?? files.FirstOrDefault();

        return new ContentVersion(
            Id: version.Id ?? string.Empty,
            Name: version.Name ?? string.Empty,
            VersionNumber: version.VersionNumber ?? string.Empty,
            GameVersions: string.Join(", ", version.GameVersions ?? new List<string>()),
            Loaders: string.Join(", ", version.Loaders ?? new List<string>()),
            DownloadUrl: string.IsNullOrWhiteSpace(file?.Url) ? null : file.Url,
            FileName: string.IsNullOrWhiteSpace(file?.Filename) ? null : file.Filename,
            FileSize: file?.Size ?? 0,
            Sha1: string.IsNullOrWhiteSpace(file?.Hashes?.Sha1) ? null : file.Hashes!.Sha1);
    }

    /// <summary>取主文件（primary=true，无则第一个）；Modpack 额外优先 .mrpack 文件，且不解析其内容。</summary>
    private static VersionFileDto? SelectDownloadFile(ContentKind kind, List<VersionFileDto>? files)
    {
        var available = (files ?? new List<VersionFileDto>())
            .Where(f => !string.IsNullOrWhiteSpace(f.Url) && !string.IsNullOrWhiteSpace(f.Filename))
            .ToList();
        if (available.Count == 0) return null;

        if (kind == ContentKind.Modpack)
        {
            var mrpack = available.FirstOrDefault(
                f => f.Filename!.EndsWith(".mrpack", StringComparison.OrdinalIgnoreCase));
            if (mrpack is not null) return mrpack;
        }

        return available.FirstOrDefault(f => f.Primary) ?? available[0];
    }

    // ---------- HTTP / 下载 ----------

    private async Task<string> GetJsonAsync(string url, CancellationToken ct)
    {
        using var request = CreateRequest(url, "application/json");
        try
        {
            using var response = await http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, ct);
            if (!response.IsSuccessStatusCode)
                throw new InvalidOperationException($"Modrinth 接口返回 {(int)response.StatusCode}：{url}");
            return await response.Content.ReadAsStringAsync(ct);
        }
        catch (OperationCanceledException) when (!ct.IsCancellationRequested)
        {
            throw new InvalidOperationException($"请求 Modrinth 超时：{url}");
        }
        catch (HttpRequestException ex)
        {
            throw new InvalidOperationException($"请求 Modrinth 失败：{url}（{ex.Message}）", ex);
        }
    }

    private async Task<string> DownloadFileAsync(
        string url,
        string destination,
        long expectedSize,
        string? sha1,
        IProgress<double>? progress,
        CancellationToken ct)
    {
        if (IsValid(destination, expectedSize, sha1))
        {
            progress?.Report(1d);
            return destination;
        }

        var directory = Path.GetDirectoryName(destination);
        if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);

        var temp = destination + ".part";

        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                using var request = CreateRequest(url, null);
                using var response = await http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, ct);
                if (!response.IsSuccessStatusCode)
                    throw new InvalidOperationException($"下载地址返回 {(int)response.StatusCode}：{url}");

                var total = response.Content.Headers.ContentLength ?? expectedSize;

                await using (var input = await response.Content.ReadAsStreamAsync(ct))
                await using (var output = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None, BufferSize, useAsync: true))
                {
                    var buffer = new byte[BufferSize];
                    long written = 0;
                    int read;
                    while ((read = await input.ReadAsync(buffer, ct)) > 0)
                    {
                        await output.WriteAsync(buffer.AsMemory(0, read), ct);
                        written += read;
                        if (total > 0) progress?.Report(Math.Clamp((double)written / total, 0d, 1d));
                    }
                }

                if (expectedSize > 0 && new FileInfo(temp).Length != expectedSize)
                    throw new InvalidOperationException($"文件大小不符（期望 {expectedSize} 字节）：{url}");

                if (sha1 is not null && !string.Equals(HashFile(temp), sha1, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException($"SHA1 校验失败：{url}");

                File.Move(temp, destination, overwrite: true);
                progress?.Report(1d);
                return destination;
            }
            catch (Exception ex) when (ex is not OperationCanceledException || !ct.IsCancellationRequested)
            {
                TryDelete(temp);
                if (attempt >= MaxAttempts)
                    throw new InvalidOperationException($"下载失败（已重试 {attempt} 次）：{url}（{ex.Message}）", ex);

                progress?.Report(0d);
                await Task.Delay(TimeSpan.FromSeconds(Math.Pow(2, attempt - 1)), ct);
            }
        }

        throw new InvalidOperationException($"下载失败：{url}"); // 逻辑上不可达
    }

    private static HttpRequestMessage CreateRequest(string url, string? accept)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);
        if (accept is not null) request.Headers.TryAddWithoutValidation("Accept", accept);
        return request;
    }

    private static T Deserialize<T>(string json, string what)
    {
        try
        {
            return JsonSerializer.Deserialize<T>(json, JsonOptions)
                   ?? throw new InvalidOperationException($"Modrinth {what}响应为空");
        }
        catch (JsonException ex)
        {
            throw new InvalidOperationException($"Modrinth {what}响应解析失败：{ex.Message}", ex);
        }
    }

    // ---------- 工具 ----------

    private static string ProjectType(ContentKind kind) => kind switch
    {
        ContentKind.Mod => "mod",
        ContentKind.Modpack => "modpack",
        ContentKind.Shader => "shader",
        ContentKind.ResourcePack => "resourcepack",
        ContentKind.DataPack => "datapack",
        _ => throw new ArgumentOutOfRangeException(nameof(kind), kind, "未知的 Modrinth 内容类型"),
    };

    private static string TargetFolder(ContentKind kind) => kind switch
    {
        ContentKind.Mod => "mods",
        ContentKind.Modpack => "modpacks",
        ContentKind.Shader => "shaderpacks",
        ContentKind.ResourcePack => "resourcepacks",
        ContentKind.DataPack => "datapacks",
        _ => throw new ArgumentOutOfRangeException(nameof(kind), kind, "未知的 Modrinth 内容类型"),
    };

    private static string SafeFileName(string? fileName, string url)
    {
        var raw = fileName;
        if (string.IsNullOrWhiteSpace(raw) && Uri.TryCreate(url, UriKind.Absolute, out var uri))
            raw = Path.GetFileName(uri.LocalPath);

        var name = string.IsNullOrWhiteSpace(raw) ? null : Path.GetFileName(raw);
        if (string.IsNullOrWhiteSpace(name))
            throw new InvalidOperationException($"无法确定下载文件名：{url}");

        foreach (var invalid in Path.GetInvalidFileNameChars())
            name = name.Replace(invalid, '_');

        return name;
    }

    private static bool IsValid(string path, long expectedSize, string? sha1)
    {
        var info = new FileInfo(path);
        if (!info.Exists) return false;
        if (expectedSize > 0 && info.Length != expectedSize) return false;
        if (sha1 is null) return true;
        return string.Equals(HashFile(path), sha1, StringComparison.OrdinalIgnoreCase);
    }

    private static string HashFile(string path)
    {
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(SHA1.HashData(stream)).ToLowerInvariant();
    }

    private static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { /* 忽略清理失败 */ }
    }

    // ---------- API DTO ----------

    private sealed class SearchResponse
    {
        public List<SearchHit>? Hits { get; set; }
    }

    private sealed class SearchHit
    {
        [JsonPropertyName("project_id")] public string? ProjectId { get; set; }
        public string? Slug { get; set; }
        public string? Title { get; set; }
        public string? Description { get; set; }
        [JsonPropertyName("icon_url")] public string? IconUrl { get; set; }
        public long Downloads { get; set; }
        public string? Author { get; set; }
    }

    private sealed class VersionDto
    {
        public string? Id { get; set; }
        public string? Name { get; set; }
        [JsonPropertyName("version_number")] public string? VersionNumber { get; set; }
        [JsonPropertyName("game_versions")] public List<string>? GameVersions { get; set; }
        public List<string>? Loaders { get; set; }
        public List<VersionFileDto>? Files { get; set; }
    }

    private sealed class VersionFileDto
    {
        public string? Url { get; set; }
        public string? Filename { get; set; }
        public bool Primary { get; set; }
        public long Size { get; set; }
        public FileHashesDto? Hashes { get; set; }
    }

    private sealed class FileHashesDto
    {
        public string? Sha1 { get; set; }
    }
}
