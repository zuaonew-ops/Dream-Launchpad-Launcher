// 作者：zuao
using System.Collections.Concurrent;
using System.Net;
using System.Text.Json;

namespace DLP.Core.Download;

/// <summary>CurseForge 整合包清单里的一个具体文件：真实文件名、大小与 CDN 下载地址。</summary>
public sealed record CurseForgeFile(
    int ProjectId,
    int FileId,
    string FileName,
    long? FileSize,
    string DownloadUrl,
    string AlternativeDownloadUrl)
{
    public override string ToString() => $"{FileName}（{DownloadUrl}）";
}

/// <summary>CurseForge 元数据请求失败（网络不可用 / 服务端异常）；「文件不在元数据里」用 <c>null</c> 表示，不抛这个异常。</summary>
public sealed class CurseForgeException(string message, Exception? inner = null) : Exception(message, inner);

/// <summary>
/// CurseForge 依赖解析：官方 api.curseforge.com 需要 API key（无 key 返回 403），
/// 这里改用 https://api.cfwidget.com（无需 key）取文件名，再按 forgecdn 规则拼出下载地址：
/// <c>https://mediafilez.forgecdn.net/files/{fileId/1000}/{fileId%1000}/{Uri.EscapeDataString(文件名)}</c>。
/// 同一个 projectID 只请求一次（失败也缓存，避免同一个项目被反复打）。
/// </summary>
public sealed class CurseForgeApi
{
    /// <summary>forgecdn 主站（实测 200，Content-Length 与 filesize 一致，支持 Range）。</summary>
    public const string CdnBase = "https://mediafilez.forgecdn.net/files";

    /// <summary>forgecdn 备用域名（同路径，实测同样可用）。</summary>
    public const string EdgeCdnBase = "https://edge.forgecdn.net/files";

    private const string WidgetBase = "https://api.cfwidget.com";
    private const int MaxAttempts = 3;
    private const string UserAgent = "DreamLaunchpad/0.4 (local launcher)";

    private static readonly Lazy<HttpClient> SharedHttp = new(CreateSharedHttp);

    private readonly HttpClient _http;

    // projectID -> 该项目全部文件的元数据；Lazy 保证同一项目并发调用只发一次请求。
    private readonly ConcurrentDictionary<int, Lazy<Task<Dictionary<int, WidgetFile>?>>> _projects = new();

    /// <param name="http">复用调用方的 HttpClient；为空时用内置共享实例（只发只读元数据请求）。</param>
    public CurseForgeApi(HttpClient? http = null) => _http = http ?? SharedHttp.Value;

    /// <summary>已请求过的项目数（诊断用）。</summary>
    public int RequestedProjects => _projects.Count;

    /// <summary>按 (projectID, fileID) 查文件名与大小；项目元数据取不到、或该 fileID 不在列表里时返回 null。</summary>
    /// <exception cref="CurseForgeException">网络不可用 / 服务端返回错误（已重试 3 次）。</exception>
    public async Task<CurseForgeFile?> ResolveAsync(int projectId, int fileId, CancellationToken ct)
    {
        var files = await GetProjectFilesAsync(projectId, ct).ConfigureAwait(false);
        if (files is null || !files.TryGetValue(fileId, out var file)) return null;

        var fileName = PickFileName(file.Name, file.Display);
        if (string.IsNullOrWhiteSpace(fileName)) return null;

        return new CurseForgeFile(projectId, fileId, fileName, file.FileSize,
            BuildDownloadUrl(CdnBase, fileId, fileName),
            BuildDownloadUrl(EdgeCdnBase, fileId, fileName));
    }

    /// <summary>拼 CDN 地址：路径分段是 fileId/1000 与 fileId%1000（不补零，实测补零会 403）。</summary>
    public static string BuildDownloadUrl(string baseUrl, int fileId, string fileName)
        => $"{baseUrl}/{fileId / 1000}/{fileId % 1000}/{Uri.EscapeDataString(fileName)}";

    // ---------------- 元数据抓取 ----------------

    private async Task<Dictionary<int, WidgetFile>?> GetProjectFilesAsync(int projectId, CancellationToken ct)
    {
        var lazy = _projects.GetOrAdd(projectId,
            id => new Lazy<Task<Dictionary<int, WidgetFile>?>>(() => FetchProjectAsync(id)));
        // 网络请求本身不挂调用方的取消令牌（结果要缓存给其它模组复用），只让等待可取消。
        return await lazy.Value.WaitAsync(ct).ConfigureAwait(false);
    }

    private async Task<Dictionary<int, WidgetFile>?> FetchProjectAsync(int projectId)
    {
        Exception? last = null;
        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
                using var request = new HttpRequestMessage(HttpMethod.Get, $"{WidgetBase}/{projectId}");
                if (_http.DefaultRequestHeaders.UserAgent.Count == 0)
                    request.Headers.TryAddWithoutValidation("User-Agent", UserAgent);

                using var response = await _http.SendAsync(request, HttpCompletionOption.ResponseContentRead, timeout.Token)
                    .ConfigureAwait(false);
                if (response.StatusCode is HttpStatusCode.NotFound or HttpStatusCode.Forbidden) return null;
                response.EnsureSuccessStatusCode();

                var json = await response.Content.ReadAsStringAsync(timeout.Token).ConfigureAwait(false);
                return ParseProject(json);
            }
            catch (Exception ex)
            {
                // 单次 30 秒超时属于可重试失败；这里统一按重试处理。
                last = ex;
                if (attempt == MaxAttempts) break;
                await Task.Delay(TimeSpan.FromMilliseconds(400 * attempt)).ConfigureAwait(false);
            }
        }

        throw new CurseForgeException($"cfwidget 元数据请求失败（已重试 {MaxAttempts - 1} 次）：projectID {projectId}\n{last?.Message}", last);
    }

    /// <summary>
    /// 解析 cfwidget 的项目 JSON：<c>{"id":n,"files":[{"id":fileId,"name":"真实文件名","display":"展示名","filesize":123}]}</c>。
    /// 旧版接口按游戏版本分组（<c>files</c> 是对象、值是数组），这里递归兼容。
    /// </summary>
    private static Dictionary<int, WidgetFile>? ParseProject(string json)
    {
        using var doc = JsonDocument.Parse(json);
        if (doc.RootElement.ValueKind != JsonValueKind.Object) return null;
        if (!doc.RootElement.TryGetProperty("files", out var filesNode)) return null;

        var result = new Dictionary<int, WidgetFile>();
        AddEntries(filesNode, result);
        return result;
    }

    private static void AddEntries(JsonElement node, Dictionary<int, WidgetFile> result)
    {
        switch (node.ValueKind)
        {
            case JsonValueKind.Array:
                foreach (var item in node.EnumerateArray()) AddEntry(item, result);
                break;

            case JsonValueKind.Object:
                // 单个文件对象 / 按版本分组两种结构：带 id 的当文件，其余当分组继续递归。
                if (node.TryGetProperty("id", out _)) AddEntry(node, result);
                else
                    foreach (var property in node.EnumerateObject())
                        AddEntries(property.Value, result);
                break;
        }
    }

    private static void AddEntry(JsonElement element, Dictionary<int, WidgetFile> result)
    {
        if (element.ValueKind != JsonValueKind.Object) return;
        if (!TryGetInt(element, "id", out var fileId)) return;
        result[fileId] = new WidgetFile(
            GetString(element, "name"),
            GetString(element, "display"),
            TryGetLong(element, "filesize"));
    }

    // ---------------- 文件名判定 ----------------

    /// <summary>
    /// 选真实文件名：cfwidget 的 <c>name</c> 一般是文件名，<c>display</c> 有时是展示标题
    /// （如 "Mantle 1.11.104 for 1.20.1"，真实文件名是 name 里的 "Mantle-1.20.1-1.11.104.jar"）。
    /// 带模组文件扩展名的一方优先，都没有扩展名时用 name。
    /// </summary>
    private static string? PickFileName(string? name, string? display)
    {
        var primary = BareName(name);
        var secondary = BareName(display);
        if (LooksLikeContentFile(primary)) return primary;
        if (LooksLikeContentFile(secondary)) return secondary;
        return primary ?? secondary;
    }

    /// <summary>去掉清单里可能带的目录前缀，只留文件名。</summary>
    private static string? BareName(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) return null;
        var bare = Path.GetFileName(value.Trim());
        return string.IsNullOrWhiteSpace(bare) ? null : bare;
    }

    private static bool LooksLikeContentFile(string? value)
    {
        if (string.IsNullOrEmpty(value)) return false;
        return value.EndsWith(".jar", StringComparison.OrdinalIgnoreCase)
               || value.EndsWith(".zip", StringComparison.OrdinalIgnoreCase)
               || value.EndsWith(".litemod", StringComparison.OrdinalIgnoreCase)
               || value.EndsWith(".jar.disabled", StringComparison.OrdinalIgnoreCase)
               || value.EndsWith(".zip.disabled", StringComparison.OrdinalIgnoreCase);
    }

    // ---------------- JSON 小工具 ----------------

    private static string? GetString(JsonElement element, string name)
        => element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static bool TryGetInt(JsonElement element, string name, out int result)
    {
        result = 0;
        if (!element.TryGetProperty(name, out var value)) return false;
        if (value.ValueKind == JsonValueKind.Number) return value.TryGetInt32(out result);
        return value.ValueKind == JsonValueKind.String && int.TryParse(value.GetString(), out result);
    }

    private static long? TryGetLong(JsonElement element, string name)
    {
        if (!element.TryGetProperty(name, out var value)) return null;
        if (value.ValueKind == JsonValueKind.Number && value.TryGetInt64(out var number)) return number;
        if (value.ValueKind == JsonValueKind.String && long.TryParse(value.GetString(), out var parsed)) return parsed;
        return null;
    }

    private static HttpClient CreateSharedHttp()
    {
        var handler = new SocketsHttpHandler
        {
            PooledConnectionLifetime = TimeSpan.FromMinutes(5),
            MaxConnectionsPerServer = 32,
            AutomaticDecompression = DecompressionMethods.All,
            ConnectTimeout = TimeSpan.FromSeconds(15),
        };
        var http = new HttpClient(handler) { Timeout = TimeSpan.FromMinutes(2) };
        http.DefaultRequestHeaders.UserAgent.ParseAdd(UserAgent);
        return http;
    }

    /// <summary>cfwidget 一个文件条目的原始字段。</summary>
    private sealed record WidgetFile(string? Name, string? Display, long? FileSize);
}
