// 作者：zuao
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using DLP.Core.Infrastructure;

namespace DLP.Core.Install;

public enum ModLoaderType { Fabric, Quilt, Forge, NeoForge }

/// <summary>某个加载器的可用版本；<see cref="MinecraftVersion"/> 为对应的原版版本。</summary>
public sealed record LoaderVersion(string Id, string DisplayName, string? MinecraftVersion);

/// <summary>加载器查询 / 安装失败（消息为中文，可直接展示给用户）。</summary>
public sealed class LoaderException(string message, Exception? inner = null) : Exception(message, inner);

/// <summary>Fabric / Quilt / Forge / NeoForge 的版本查询与安装。</summary>
public sealed class LoaderService(GamePaths paths, HttpClient http)
{
    private const int MaxAttempts = 3; // 首次 + 2 次重试
    private const string Bmclapi = "https://bmclapi2.bangbang93.com";
    private const string FabricMeta = "https://meta.fabricmc.net/v2";
    private const string QuiltMeta = "https://meta.quiltmc.org/v3";

    private static readonly TimeSpan InstallerTimeout = TimeSpan.FromMinutes(10);
    private static readonly JsonSerializerOptions IndentedJson = new() { WriteIndented = true };

    /// <summary>查询加载器可用版本；Forge / NeoForge 按版本号降序（Fabric / Quilt 保留官方顺序，最新在前）。</summary>
    public async Task<IReadOnlyList<LoaderVersion>> ListAsync(ModLoaderType type, string mcVersion, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(mcVersion);

        return type switch
        {
            ModLoaderType.Fabric => await ListFabricAsync(mcVersion, ct),
            ModLoaderType.Quilt => await ListQuiltAsync(mcVersion, ct),
            ModLoaderType.Forge => await ListForgeAsync(mcVersion, ct),
            ModLoaderType.NeoForge => await ListNeoForgeAsync(mcVersion, ct),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "未知的加载器类型")
        };
    }

    /// <summary>安装加载器，返回安装出的版本 id。</summary>
    public async Task<string> InstallAsync(ModLoaderType type, string mcVersion, string loaderVersion,
        IProgress<string>? status, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(mcVersion);
        ArgumentException.ThrowIfNullOrWhiteSpace(loaderVersion);

        return type switch
        {
            ModLoaderType.Fabric => await InstallProfileAsync("fabric-loader", FabricMeta, mcVersion, loaderVersion, status, ct),
            ModLoaderType.Quilt => await InstallProfileAsync("quilt-loader", QuiltMeta, mcVersion, loaderVersion, status, ct),
            ModLoaderType.Forge => await InstallWithInstallerAsync(ModLoaderType.Forge, mcVersion, loaderVersion, status, ct),
            ModLoaderType.NeoForge => await InstallWithInstallerAsync(ModLoaderType.NeoForge, mcVersion, loaderVersion, status, ct),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "未知的加载器类型")
        };
    }

    // ---------------- 版本列表 ----------------

    private async Task<IReadOnlyList<LoaderVersion>> ListFabricAsync(string mcVersion, CancellationToken ct)
    {
        var json = await GetStringAsync($"{FabricMeta}/versions/loader/{Uri.EscapeDataString(mcVersion)}", ct);
        using var doc = ParseArray(json, "Fabric 版本列表");

        var result = new List<LoaderVersion>();
        foreach (var entry in doc.RootElement.EnumerateArray())
        {
            if (!entry.TryGetProperty("loader", out var loader)) continue;
            if (GetStringOrNull(loader, "version") is not { Length: > 0 } version) continue;

            var stable = !(loader.TryGetProperty("stable", out var flag) && flag.ValueKind == JsonValueKind.False);
            result.Add(new LoaderVersion(version,
                stable ? $"Fabric Loader {version}" : $"Fabric Loader {version}（测试版）", mcVersion));
        }
        return result;
    }

    private async Task<IReadOnlyList<LoaderVersion>> ListQuiltAsync(string mcVersion, CancellationToken ct)
    {
        var json = await GetStringAsync($"{QuiltMeta}/versions/loader/{Uri.EscapeDataString(mcVersion)}", ct);
        using var doc = ParseArray(json, "Quilt 版本列表");

        var result = new List<LoaderVersion>();
        foreach (var entry in doc.RootElement.EnumerateArray())
        {
            if (!entry.TryGetProperty("loader", out var loader)) continue;
            if (GetStringOrNull(loader, "version") is not { Length: > 0 } version) continue;

            result.Add(new LoaderVersion(version, $"Quilt Loader {version}", mcVersion));
        }
        return result;
    }

    private async Task<IReadOnlyList<LoaderVersion>> ListForgeAsync(string mcVersion, CancellationToken ct)
    {
        var json = await GetStringAsync($"{Bmclapi}/forge/minecraft/{Uri.EscapeDataString(mcVersion)}", ct);
        using var doc = ParseArray(json, "Forge 版本列表");

        var versions = new List<string>();
        foreach (var entry in doc.RootElement.EnumerateArray())
        {
            if (NormalizeForgeVersion(mcVersion, GetStringOrNull(entry, "version")) is { Length: > 0 } version)
                versions.Add(version);
        }

        return SortDescending(versions)
            .Select(v => new LoaderVersion(v, $"Forge {v}", mcVersion))
            .ToList();
    }

    private async Task<IReadOnlyList<LoaderVersion>> ListNeoForgeAsync(string mcVersion, CancellationToken ct)
    {
        var json = await GetStringAsync($"{Bmclapi}/neoforge/list/{Uri.EscapeDataString(mcVersion)}", ct);
        using var doc = ParseArray(json, "NeoForge 版本列表");

        var versions = new List<string>();
        foreach (var entry in doc.RootElement.EnumerateArray())
        {
            if (GetStringOrNull(entry, "version") is { Length: > 0 } version) versions.Add(version);
        }

        return SortDescending(versions)
            .Select(v => new LoaderVersion(v, $"NeoForge {v}", mcVersion))
            .ToList();
    }

    private static IEnumerable<string> SortDescending(IEnumerable<string> versions) =>
        versions.Distinct(StringComparer.OrdinalIgnoreCase)
                .OrderByDescending(v => v, LoaderVersionOrder.Instance);

    /// <summary>个别镜像把 Forge 版本写成 "1.20.1-47.0.1"，去掉原版前缀保持 "47.0.1"。</summary>
    private static string? NormalizeForgeVersion(string mcVersion, string? version)
    {
        if (string.IsNullOrWhiteSpace(version)) return null;
        var trimmed = version.Trim();
        var prefix = mcVersion + "-";
        return trimmed.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) ? trimmed[prefix.Length..] : trimmed;
    }

    // ---------------- 安装：Fabric / Quilt（改写 profile JSON） ----------------

    /// <summary>取官方 profile JSON，把 id 改成 &lt;前缀&gt;-&lt;加载器版本&gt;-&lt;原版版本&gt; 后写入 versions/&lt;id&gt;/&lt;id&gt;.json。</summary>
    private async Task<string> InstallProfileAsync(string idPrefix, string metaBase, string mcVersion, string loaderVersion,
        IProgress<string>? status, CancellationToken ct)
    {
        var newId = $"{idPrefix}-{loaderVersion}-{mcVersion}";
        var url = $"{metaBase}/versions/loader/{Uri.EscapeDataString(mcVersion)}/{Uri.EscapeDataString(loaderVersion)}/profile/json";

        status?.Report($"正在获取 {newId} 的安装配置");
        var json = await GetStringAsync(url, ct);

        JsonObject profile;
        try
        {
            profile = JsonNode.Parse(json) as JsonObject ?? throw new JsonException("配置不是 JSON 对象");
        }
        catch (JsonException ex)
        {
            throw new LoaderException($"解析 {newId} 安装配置失败：{ex.Message}", ex);
        }

        profile["id"] = newId;
        // 保留原有 inheritsFrom；仅在缺失时补上原版版本，保证父版本可解析。
        if (profile["inheritsFrom"] is not JsonValue inheritValue
            || !inheritValue.TryGetValue<string>(out var inheritId)
            || string.IsNullOrWhiteSpace(inheritId))
        {
            profile["inheritsFrom"] = mcVersion;
        }

        var dir = paths.VersionDir(newId);
        Directory.CreateDirectory(dir);
        var target = paths.VersionJson(newId);
        await File.WriteAllTextAsync(target, profile.ToJsonString(IndentedJson), ct);

        status?.Report($"已写入 {target}");
        return newId;
    }

    // ---------------- 安装：Forge / NeoForge（下载并运行 installer） ----------------

    private async Task<string> InstallWithInstallerAsync(ModLoaderType type, string mcVersion, string loaderVersion,
        IProgress<string>? status, CancellationToken ct)
    {
        var isForge = type == ModLoaderType.Forge;
        var prefix = isForge ? "forge" : "neoforge";
        var newId = $"{prefix}-{mcVersion}-{loaderVersion}";

        var urls = isForge
            ? ForgeInstallerUrls(mcVersion, loaderVersion)
            : await NeoForgeInstallerUrlsAsync(mcVersion, loaderVersion, ct);
        var installerPath = Path.Combine(paths.Cache, $"{prefix}-installer-{mcVersion}-{loaderVersion}.jar");

        var downloaded = false;
        LoaderException? lastError = null;
        foreach (var url in urls)
        {
            try
            {
                status?.Report($"正在下载 {prefix} {loaderVersion} 安装器");
                await DownloadFileAsync(url, installerPath, status, ct);
                downloaded = true;
                break;
            }
            catch (LoaderException ex)
            {
                lastError = ex;
                status?.Report($"该地址下载失败，换下一个：{ex.Message}");
            }
        }

        if (!downloaded)
            throw new LoaderException($"下载 {prefix} {loaderVersion} 安装器失败：所有地址都不可用", lastError);

        var javaExe = FindJava(mcVersion) ?? throw new LoaderException(
            $"未找到 Java：{prefix} {loaderVersion} 安装器需要 Java 才能运行。"
            + "请安装 Java，或设置 JAVA_HOME / 将 java.exe 所在目录加入 PATH 后重试。");

        EnsureLauncherProfiles(paths.Root);

        var before = SnapshotVersionDirs();
        status?.Report($"正在运行 {prefix} 安装器（最长 {InstallerTimeout.TotalMinutes:0} 分钟）");
        await RunInstallerAsync(javaExe, installerPath, paths.Root, status, ct);

        // 安装器实际写出的目录名与规范 id 不一定一致（Forge 建的是 <mc>-forge-<ver>），以目录为准。
        var actual = SnapshotVersionDirs().FirstOrDefault(name => !before.Contains(name)) ?? newId;
        status?.Report($"{prefix} {loaderVersion} 安装完成（{actual}）");

        return actual;
    }

    private HashSet<string> SnapshotVersionDirs()
        => Directory.Exists(paths.Versions)
            ? Directory.GetDirectories(paths.Versions).Select(Path.GetFileName).Where(n => n is not null).Select(n => n!).ToHashSet(StringComparer.OrdinalIgnoreCase)
            : new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    /// <summary>Forge / NeoForge 安装器要求目标目录已有 launcher_profiles.json，没有就直接报错；缺失时补一个空档案。</summary>
    private static void EnsureLauncherProfiles(string root)
    {
        var path = Path.Combine(root, "launcher_profiles.json");
        if (File.Exists(path)) return;

        File.WriteAllText(path, """
            {
              "profiles": {},
              "settings": {},
              "version": 3
            }
            """);
    }

    private static IReadOnlyList<string> ForgeInstallerUrls(string mcVersion, string loaderVersion)
    {
        var mc = Uri.EscapeDataString(mcVersion);
        var ver = Uri.EscapeDataString(loaderVersion);
        return
        [
            $"{Bmclapi}/forge/download?mcversion={mc}&version={ver}&category=installer&format=jar",
            $"{Bmclapi}/maven/net/minecraftforge/forge/{mcVersion}-{loaderVersion}/forge-{mcVersion}-{loaderVersion}-installer.jar"
        ];
    }

    /// <summary>NeoForge 安装器地址：优先用版本列表里的 installerPath，其次按 Maven 路径推测（含 1.20.1 旧布局）。</summary>
    private async Task<IReadOnlyList<string>> NeoForgeInstallerUrlsAsync(string mcVersion, string loaderVersion, CancellationToken ct)
    {
        var urls = new List<string>();
        try
        {
            var json = await GetStringAsync($"{Bmclapi}/neoforge/list/{Uri.EscapeDataString(mcVersion)}", ct);
            using var doc = ParseArray(json, "NeoForge 版本列表");
            foreach (var entry in doc.RootElement.EnumerateArray())
            {
                if (!string.Equals(GetStringOrNull(entry, "version"), loaderVersion, StringComparison.OrdinalIgnoreCase)) continue;
                if (GetStringOrNull(entry, "installerPath") is { Length: > 0 } installerPath)
                    urls.Add(installerPath.StartsWith('/') ? Bmclapi + installerPath : Bmclapi + "/" + installerPath);
                break;
            }
        }
        catch (LoaderException)
        {
            // 列表拿不到时直接走下面的推测地址
        }

        var mc = Uri.EscapeDataString(mcVersion);
        var ver = Uri.EscapeDataString(loaderVersion);
        urls.Add($"{Bmclapi}/maven/net/neoforged/neoforge/{ver}/neoforge-{ver}-installer.jar");
        urls.Add($"{Bmclapi}/maven/net/neoforged/forge/{mcVersion}-{loaderVersion}/forge-{mcVersion}-{loaderVersion}-installer.jar");
        urls.Add($"{Bmclapi}/forge/download?mcversion={mc}&version={ver}&category=installer&format=jar");

        return urls.Distinct(StringComparer.OrdinalIgnoreCase).ToList();
    }

    private async Task DownloadFileAsync(string url, string destination, IProgress<string>? status, CancellationToken ct)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
        var temp = destination + ".dlp-part";
        Exception? last = null;

        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                status?.Report($"下载 {Path.GetFileName(destination)}（第 {attempt}/{MaxAttempts} 次）");
                using var request = CreateRequest(url);
                using var response = await http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, ct);
                response.EnsureSuccessStatusCode();
                await using (var input = await response.Content.ReadAsStreamAsync(ct))
                await using (var output = File.Create(temp))
                    await input.CopyToAsync(output, ct);

                File.Move(temp, destination, overwrite: true);
                return;
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                last = ex;
                TryDelete(temp);
                if (attempt == MaxAttempts) break;
                status?.Report($"下载失败，{attempt} 秒后重试：{ex.Message}");
                await Task.Delay(TimeSpan.FromSeconds(attempt), ct);
            }
        }

        throw new LoaderException($"下载失败（已重试 {MaxAttempts - 1} 次）：{url}\n{last?.Message}", last);
    }

    private static async Task RunInstallerAsync(string javaExe, string installerPath, string targetDir,
        IProgress<string>? status, CancellationToken ct)
    {
        var startInfo = new ProcessStartInfo(javaExe)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            WorkingDirectory = Path.GetDirectoryName(installerPath) ?? AppContext.BaseDirectory
        };
        startInfo.ArgumentList.Add("-jar");
        startInfo.ArgumentList.Add(installerPath);
        startInfo.ArgumentList.Add("--installClient");
        startInfo.ArgumentList.Add(targetDir);

        var output = new StringBuilder();
        using var process = new Process { StartInfo = startInfo };
        process.OutputDataReceived += (_, e) => AppendOutput(output, e.Data, status);
        process.ErrorDataReceived += (_, e) => AppendOutput(output, e.Data, status);

        try
        {
            if (!process.Start()) throw new LoaderException($"无法启动 Java 进程：{javaExe}");
        }
        catch (LoaderException) { throw; }
        catch (Exception ex)
        {
            throw new LoaderException($"无法启动 Java 进程（{javaExe}）：{ex.Message}", ex);
        }

        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeout.CancelAfter(InstallerTimeout);
        try
        {
            await process.WaitForExitAsync(timeout.Token);
        }
        catch (OperationCanceledException)
        {
            TryKill(process);
            if (ct.IsCancellationRequested) throw;
            throw new LoaderException($"安装器执行超时（{InstallerTimeout.TotalMinutes:0} 分钟）：{Path.GetFileName(installerPath)}");
        }

        if (process.ExitCode != 0)
        {
            var text = output.ToString();
            // 旧版 NeoForge 1.20.1 安装器只有服务端安装选项，会报 "installClient is not a recognized option"。
            if (text.Contains("installClient", StringComparison.OrdinalIgnoreCase)
                && text.Contains("not a recognized option", StringComparison.OrdinalIgnoreCase))
            {
                throw new LoaderException(
                    $"该安装器不支持客户端安装（旧版安装器只支持服务端）：{Path.GetFileName(installerPath)}。"
                    + "请选择更新的加载器版本后重试。");
            }

            throw new LoaderException($"安装器返回错误（退出码 {process.ExitCode}）：{Path.GetFileName(installerPath)}\n{Tail(text, 20)}");
        }
    }

    // ---------------- 查找 Java ----------------

    /// <summary>按 JAVA_HOME、PATH、内置 runtime 目录找 java.exe，并优先选接近该原版版本所需主版本的运行时。</summary>
    private string? FindJava(string mcVersion)
    {
        var candidates = new List<string>();

        if (Environment.GetEnvironmentVariable("JAVA_HOME") is { Length: > 0 } javaHome)
            candidates.Add(Path.Combine(javaHome.Trim().Trim('"'), "bin", "java.exe"));

        var pathVar = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var dir in pathVar.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries))
        {
            if (string.IsNullOrWhiteSpace(dir)) continue;
            candidates.Add(Path.Combine(dir.Trim().Trim('"'), "java.exe"));
        }

        try { candidates.AddRange(Directory.EnumerateFiles(paths.Runtime, "java.exe", SearchOption.AllDirectories)); }
        catch (IOException) { /* 运行时目录不可读时忽略 */ }
        catch (UnauthorizedAccessException) { /* 同上 */ }

        var found = candidates
            .Where(File.Exists)
            .Select(path => (Path: Path.GetFullPath(path), Major: ProbeJavaMajor(path)))
            .ToList();
        if (found.Count == 0) return null;

        if (RequiredJavaMajor(mcVersion) is { } required)
        {
            var exact = found.FirstOrDefault(f => f.Major == required);
            if (exact.Path is not null) return exact.Path;

            var newer = found.Where(f => f.Major >= required).OrderBy(f => f.Major).FirstOrDefault();
            if (newer.Path is not null) return newer.Path;
        }

        return found.OrderByDescending(f => f.Major).First().Path;
    }

    /// <summary>读取 java.exe 上级目录的 release 文件判断主版本；读不到返回 0（未知）。</summary>
    private static int ProbeJavaMajor(string javaExe)
    {
        try
        {
            var home = Path.GetFullPath(Path.Combine(Path.GetDirectoryName(javaExe)!, ".."));
            var release = Path.Combine(home, "release");
            if (!File.Exists(release)) return 0;

            foreach (var line in File.ReadLines(release))
            {
                if (!line.StartsWith("JAVA_VERSION=", StringComparison.Ordinal)) continue;
                return ParseJavaMajor(line["JAVA_VERSION=".Length..].Trim().Trim('"'));
            }
        }
        catch (IOException) { /* 忽略 */ }
        catch (UnauthorizedAccessException) { /* 忽略 */ }
        return 0;
    }

    private static int ParseJavaMajor(string version)
    {
        if (version.StartsWith("1.", StringComparison.Ordinal))
            return int.TryParse(version.Split('.')[1], out var legacy) ? legacy : 8;
        return int.TryParse(version.Split('.')[0], out var major) ? major : 0;
    }

    /// <summary>按原版版本推断安装器所需 Java 主版本；无法判断时返回 null（用最新 Java）。</summary>
    private static int? RequiredJavaMajor(string mcVersion)
    {
        var parts = mcVersion.Split('.');
        if (parts.Length < 2 || !int.TryParse(parts[1], out var minor)) return null;
        var patch = parts.Length > 2 && int.TryParse(parts[2], out var parsed) ? parsed : 0;

        if (minor >= 21) return 21;
        if (minor == 20 && patch >= 5) return 21;
        if (minor >= 18) return 17;
        if (minor == 17) return 16;
        return 8;
    }

    // ---------------- 小工具 ----------------

    /// <summary>
    /// 构造请求并补上 User-Agent：BMCLAPI 会轮询多个镜像，部分镜像拒绝对无 UA 的请求（返回 403），
    /// 调用方已设置默认 UA 时不再覆盖。
    /// </summary>
    private HttpRequestMessage CreateRequest(string url)
    {
        var request = new HttpRequestMessage(HttpMethod.Get, url);
        if (http.DefaultRequestHeaders.UserAgent.Count == 0)
            request.Headers.TryAddWithoutValidation("User-Agent", "DreamLaunchpad/1.0 (+https://github.com/dlp)");
        return request;
    }

    private async Task<string> GetStringAsync(string url, CancellationToken ct)
    {
        Exception? last = null;
        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                using var request = CreateRequest(url);
                using var response = await http.SendAsync(request, ct);
                response.EnsureSuccessStatusCode();
                return await response.Content.ReadAsStringAsync(ct);
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                last = ex;
                if (attempt == MaxAttempts) break;
                await Task.Delay(TimeSpan.FromMilliseconds(400 * attempt), ct);
            }
        }

        throw new LoaderException($"网络请求失败（已重试 {MaxAttempts - 1} 次）：{url}\n{last?.Message}", last);
    }

    /// <summary>读取字符串属性；属性缺失或不是字符串时返回 null。</summary>
    private static string? GetStringOrNull(JsonElement element, string propertyName) =>
        element.ValueKind == JsonValueKind.Object
        && element.TryGetProperty(propertyName, out var value)
        && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static JsonDocument ParseJson(string json, string source)
    {
        try { return JsonDocument.Parse(json); }
        catch (JsonException ex) { throw new LoaderException($"{source} 返回的数据无法解析：{ex.Message}", ex); }
    }

    private static JsonDocument ParseArray(string json, string source)
    {
        var doc = ParseJson(json, source);
        if (doc.RootElement.ValueKind == JsonValueKind.Array) return doc;
        doc.Dispose();
        throw new LoaderException($"{source} 返回的不是数组");
    }

    private static void AppendOutput(StringBuilder sink, string? line, IProgress<string>? status)
    {
        if (string.IsNullOrWhiteSpace(line)) return;
        lock (sink) sink.AppendLine(line);
        status?.Report(line);
    }

    private static string Tail(string text, int lines)
    {
        var all = text.Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        return all.Length <= lines ? string.Join('\n', all) : string.Join('\n', all[^lines..]);
    }

    private static void TryKill(Process process)
    {
        try { if (!process.HasExited) process.Kill(entireProcessTree: true); }
        catch { /* 忽略 */ }
    }

    private static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); }
        catch { /* 忽略 */ }
    }

    /// <summary>按点分隔的段比较版本：数字段按数值比较且优先于文字段；多出数字段表示更新，多出文字段表示预发布。</summary>
    private sealed class LoaderVersionOrder : IComparer<string>
    {
        public static readonly LoaderVersionOrder Instance = new();

        public int Compare(string? x, string? y) => CompareVersions(x ?? "", y ?? "");
    }

    private static int CompareVersions(string left, string right)
    {
        var a = SplitVersion(left);
        var b = SplitVersion(right);

        for (var i = 0; i < Math.Min(a.Length, b.Length); i++)
        {
            var aNumeric = int.TryParse(a[i], out var aNumber);
            var bNumeric = int.TryParse(b[i], out var bNumber);

            int cmp;
            if (aNumeric && bNumeric) cmp = aNumber.CompareTo(bNumber);
            else if (aNumeric) cmp = 1;          // 数字段 > 文字段（正式版 > 预发布）
            else if (bNumeric) cmp = -1;
            else cmp = string.Compare(a[i], b[i], StringComparison.OrdinalIgnoreCase);

            if (cmp != 0) return cmp;
        }

        if (a.Length != b.Length)
        {
            var longer = a.Length > b.Length ? a : b;
            var longerIsNewer = int.TryParse(longer[Math.Min(a.Length, b.Length)], out _);
            var cmp = longerIsNewer ? 1 : -1;
            return a.Length > b.Length ? cmp : -cmp;
        }

        return string.Compare(left, right, StringComparison.OrdinalIgnoreCase);
    }

    private static string[] SplitVersion(string version) =>
        version.Split(['.', '-', '+', '_'], StringSplitOptions.RemoveEmptyEntries);
}


