// 整合包安装：Modrinth（.mrpack，根目录 modrinth.index.json）与 CurseForge（zip，根目录 manifest.json）两种格式。
// 解析清单 → 安装基础版本与加载器 → 落盘内容文件与覆盖文件。
using System.IO.Compression;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using DLP.Core.Download;
using DLP.Core.Infrastructure;

namespace DLP.Core.Install;

/// <summary>整合包安装结果；<see cref="FailedFiles"/> 为 CurseForge 整合包里下载失败的模组数（Modrinth 格式目前失败即抛异常）。</summary>
public sealed record ModpackInstallResult(
    string VersionId,
    string VersionName,
    string GameDir,
    int FilesInstalled,
    int OverridesInstalled,
    int FailedFiles = 0);

/// <summary>Modrinth 整合包安装：mrpack 下载 / 解压 / 基础版本 / 加载器 / 内容文件 / 覆盖文件。</summary>
public sealed class ModpackInstaller(
    GamePaths paths,
    DownloadClient downloader,
    GameInstaller installer,
    LoaderService loaders,
    ModrinthService modrinth,
    Log log)
{
    private HttpClient? _http;
    private CurseForgeInstaller? _curseForge;

    /// <summary>兼容按 (paths, http, downloader, installer, loaders, modrinth, log) 构造的调用方；http 由 downloader / modrinth / CurseForge 元数据复用。</summary>
    public ModpackInstaller(GamePaths paths, HttpClient http, DownloadClient downloader, GameInstaller installer,
        LoaderService loaders, ModrinthService modrinth, Log log)
        : this(paths, downloader, installer, loaders, modrinth, log)
    {
        ArgumentNullException.ThrowIfNull(http);
        _http = http;
    }

    /// <summary>CurseForge 格式安装器：第一次用到时才创建（App 走 6 参构造，由它自己建元数据用的 HttpClient）。</summary>
    private CurseForgeInstaller CurseForge => _curseForge ??= new CurseForgeInstaller(
        paths, downloader, installer, loaders, new CurseForgeApi(_http), log);

    private const string IndexFileName = "modrinth.index.json";
    // 中文名与 "+08:00" 时区偏移按原样写出，不转义成 \uXXXX。
    private static readonly JsonSerializerOptions IndentedJson = new()
    {
        WriteIndented = true,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    /// <summary>
    /// 安装整合包。<paramref name="slug"/> 为 Modrinth 项目 slug（如 fabulously-optimized），
    /// <paramref name="loader"/> 为空时按整合包 dependencies 自动判断加载器。
    /// </summary>
    public async Task<ModpackInstallResult> InstallAsync(string slug, string mcVersion, string? loader,
        IProgress<string>? status, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(slug);
        ArgumentException.ThrowIfNullOrWhiteSpace(mcVersion);

        status?.Report("获取整合包信息");
        var versions = await modrinth.GetVersionsAsync(ContentKind.Modpack, slug, mcVersion, loader, ct);
        if (versions.Count == 0)
        {
            var loaderText = string.IsNullOrWhiteSpace(loader) ? string.Empty : $"，加载器 {loader}";
            throw new InvalidOperationException($"Modrinth 上未找到整合包：{slug}（游戏版本 {mcVersion}{loaderText}）");
        }

        var contentVersion = versions[0];
        if (string.IsNullOrWhiteSpace(contentVersion.DownloadUrl))
            throw new InvalidOperationException($"整合包 {slug} 的版本 {contentVersion.VersionNumber} 没有可下载的 .mrpack 文件");

        var fileName = string.IsNullOrWhiteSpace(contentVersion.FileName) ? slug + ".mrpack" : contentVersion.FileName!;
        var mrpackPath = Path.Combine(paths.Cache, SafeFileName(fileName));

        status?.Report("下载整合包文件");
        log.Info($"下载整合包 {slug} {contentVersion.VersionNumber}：{contentVersion.DownloadUrl}");
        await downloader.DownloadAsync(contentVersion.DownloadUrl!, mrpackPath,
            contentVersion.Sha1, contentVersion.FileSize, ct);

        var tempDir = CreateTempDirectory();
        try
        {
            ZipFile.ExtractToDirectory(mrpackPath, tempDir, overwriteFiles: true);

            var index = ReadIndex(tempDir, slug);
            var name = GetString(index["name"]) ?? slug;
            var modpackVersion = GetString(index["versionId"]) ?? contentVersion.VersionNumber;
            if (string.IsNullOrWhiteSpace(modpackVersion)) modpackVersion = contentVersion.Id;

            var dependencies = index["dependencies"] as JsonObject;
            var packMcVersion = GetString(dependencies?["minecraft"]);
            if (!string.IsNullOrWhiteSpace(packMcVersion)
                && !string.Equals(packMcVersion, mcVersion, StringComparison.OrdinalIgnoreCase))
            {
                log.Warn($"整合包声明的游戏版本是 {packMcVersion}，与命令参数 {mcVersion} 不一致，按参数安装");
            }

            return await InstallExtractedAsync(tempDir, index, slug, name, modpackVersion, mcVersion, "modrinth", status, ct);
        }
        finally
        {
            TryDeleteDirectory(tempDir);
        }
    }

    /// <summary>
    /// 安装本地整合包文件，按压缩包内容自动分派：
    /// 根目录有 modrinth.index.json → Modrinth 格式（.mrpack，或改名为 .zip 的同结构包）；
    /// 根目录有 manifest.json 且 manifestType=minecraftModpack → CurseForge 格式（manifest.json + overrides/）。
    /// 游戏版本优先取清单声明，缺失时用 <paramref name="mcVersionHint"/>。
    /// </summary>
    public async Task<ModpackInstallResult> InstallFromFileAsync(string mrpackPath, string? mcVersionHint,
        IProgress<string>? status, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(mrpackPath);
        var fullPath = Path.GetFullPath(mrpackPath);
        if (!File.Exists(fullPath))
            throw new FileNotFoundException("找不到整合包文件", fullPath);

        status?.Report("读取整合包清单");
        switch (DetectFormat(fullPath))
        {
            case PackArchiveFormat.CurseForge:
                log.Info($"识别为 CurseForge 整合包：{fullPath}");
                var curseForgeResult = await CurseForge.InstallAsync(fullPath, mcVersionHint, status, ct);
                return new ModpackInstallResult(curseForgeResult.VersionId, curseForgeResult.Name,
                    curseForgeResult.GameDir, curseForgeResult.ModsInstalled, curseForgeResult.OverridesInstalled,
                    curseForgeResult.ModsFailed);

            case PackArchiveFormat.Unknown:
                throw new InvalidOperationException("不是有效的整合包压缩包（既没有 modrinth.index.json，也没有 CurseForge 的 manifest.json）");
        }

        status?.Report("解压整合包");
        var tempDir = CreateTempDirectory();
        try
        {
            ZipFile.ExtractToDirectory(fullPath, tempDir, overwriteFiles: true);
            if (!File.Exists(Path.Combine(tempDir, IndexFileName)))
                throw new InvalidOperationException("不是有效的整合包压缩包");

            var fileName = Path.GetFileNameWithoutExtension(fullPath);
            var index = ReadIndex(tempDir, fileName);
            var name = GetString(index["name"]);
            if (string.IsNullOrWhiteSpace(name)) name = fileName;
            var slug = MakeSlug(name);
            var modpackVersion = GetString(index["versionId"]);
            if (string.IsNullOrWhiteSpace(modpackVersion)) modpackVersion = "1.0.0";

            var dependencies = index["dependencies"] as JsonObject;
            var mcVersion = GetString(dependencies?["minecraft"]);
            if (string.IsNullOrWhiteSpace(mcVersion)) mcVersion = mcVersionHint;
            if (string.IsNullOrWhiteSpace(mcVersion))
                throw new InvalidOperationException("整合包缺少 minecraft 依赖，无法确定游戏版本");

            log.Info($"本地整合包 {fullPath}：{name} {modpackVersion}，游戏版本 {mcVersion}");
            return await InstallExtractedAsync(tempDir, index, slug, name, modpackVersion, mcVersion, "local", status, ct);
        }
        catch (InvalidDataException ex)
        {
            throw new InvalidOperationException("不是有效的整合包压缩包", ex);
        }
        finally
        {
            TryDeleteDirectory(tempDir);
        }
    }

    // ---------------- 安装流程 ----------------

    /// <summary>解压并读取清单后的共用流程：基础版本 → 内容文件 → 覆盖文件 → 全部成功后才写版本 json。</summary>
    private async Task<ModpackInstallResult> InstallExtractedAsync(string tempDir, JsonObject index, string slug,
        string name, string modpackVersion, string mcVersion, string source, IProgress<string>? status,
        CancellationToken ct)
    {
        var dependencies = index["dependencies"] as JsonObject;
        var versionId = MakeVersionId(slug, modpackVersion);
        var baseVersionId = await EnsureBaseVersionAsync(mcVersion, dependencies, status, ct);

        var gameDir = paths.GameDir(versionId);
        Directory.CreateDirectory(gameDir);
        log.Info($"整合包版本 {versionId} 继承 {baseVersionId}，游戏目录 {gameDir}");

        var (installed, failed) = await DownloadFilesAsync(index, versionId, gameDir, status, ct);

        status?.Report("复制覆盖文件");
        var overrides = CopyOverrides(tempDir, gameDir);
        log.Info($"覆盖文件复制完成：{overrides} 个");

        if (failed > 0)
        {
            log.Error($"整合包 {versionId} 有 {failed} 个内容文件下载失败");
            throw new InvalidOperationException(
                $"整合包 {versionId} 有 {failed} 个内容文件下载失败（已成功 {installed} 个），可重新执行同一命令续传");
        }

        // 全部成功后才写版本 json，避免半成品版本出现在版本列表里。
        WriteVersionJson(versionId, baseVersionId, name, slug, source);
        status?.Report($"安装完成 {versionId}");
        log.Info($"安装完成 {versionId}：{installed} 个文件，{overrides} 个覆盖文件");
        return new ModpackInstallResult(versionId, name, gameDir, installed, overrides);
    }

    // ---------------- 基础版本 / 加载器 ----------------

    /// <summary>确保原版可用，再按 dependencies 安装加载器；返回可继承的基础版本 id。</summary>
    private async Task<string> EnsureBaseVersionAsync(string mcVersion, JsonObject? dependencies,
        IProgress<string>? status, CancellationToken ct)
    {
        status?.Report("安装基础版本");
        if (!File.Exists(paths.VersionJson(mcVersion)))
        {
            log.Info($"未检测到原版 {mcVersion}，先安装");
            await installer.InstallAsync(mcVersion, null, ct, asDependency: true);
        }

        var loaderSpecs = new (string Key, ModLoaderType Type)[]
        {
            ("fabric-loader", ModLoaderType.Fabric),
            ("quilt-loader", ModLoaderType.Quilt),
            ("forge", ModLoaderType.Forge),
            ("neoforge", ModLoaderType.NeoForge),
        };

        var found = loaderSpecs
            .Select(spec => (spec.Type, Version: GetString(dependencies?[spec.Key])))
            .Where(item => !string.IsNullOrWhiteSpace(item.Version))
            .ToList();

        if (found.Count == 0)
        {
            log.Info($"整合包未声明加载器依赖，使用原版 {mcVersion}");
            return mcVersion;
        }

        if (found.Count > 1)
            log.Warn($"整合包声明了多个加载器依赖（{string.Join("、", found.Select(f => f.Type))}），只安装第一个");

        var (type, loaderVersion) = found[0];
        log.Info($"安装加载器 {type} {loaderVersion}");
        var loaderId = await loaders.InstallAsync(type, mcVersion, loaderVersion!, status, ct);
        log.Info($"加载器安装完成：{loaderId}");
        return loaderId;
    }

    // ---------------- 版本 json ----------------

    private void WriteVersionJson(string versionId, string baseVersionId, string name, string slug, string source)
    {
        var now = DateTimeOffset.Now.ToString("o");
        var json = new JsonObject
        {
            ["id"] = versionId,
            ["type"] = "release",
            ["inheritsFrom"] = baseVersionId,
            ["time"] = now,
            ["releaseTime"] = now,
            ["x-dlp-modpack"] = new JsonObject
            {
                ["name"] = name,
                ["slug"] = slug,
                ["source"] = source,
            },
        };

        Directory.CreateDirectory(paths.VersionDir(versionId));
        var target = paths.VersionJson(versionId);
        File.WriteAllText(target, json.ToJsonString(IndentedJson));
        log.Info($"写入版本描述：{target}");
    }

    // ---------------- 内容文件 ----------------

    private async Task<(int Installed, int Failed)> DownloadFilesAsync(JsonObject index, string versionId,
        string gameDir, IProgress<string>? status, CancellationToken ct)
    {
        var files = index["files"] as JsonArray;
        var total = files?.Count ?? 0;
        var installed = 0;
        var failed = 0;
        var skipped = 0;
        var done = 0;

        // 并发下载：CDN 单个连接很慢，串行装 51 个文件要十几分钟。
        using var gate = new SemaphoreSlim(16);
        var tasks = new List<Task>();

        for (var i = 0; i < total; i++)
        {
            ct.ThrowIfCancellationRequested();
            if (files![i] is not JsonObject file)
            {
                skipped++;
                log.Warn($"跳过第 {i + 1} 个条目：格式不是对象");
                continue;
            }

            var relativePath = GetString(file["path"]);
            if (string.IsNullOrWhiteSpace(relativePath))
            {
                skipped++;
                log.Warn($"跳过第 {i + 1} 个条目：缺少 path");
                continue;
            }

            if (string.Equals(GetString(file["env"]?["client"]), "unsupported", StringComparison.OrdinalIgnoreCase))
            {
                skipped++;
                log.Warn($"跳过仅服务端文件：{relativePath}");
                continue;
            }

            var url = FirstDownloadUrl(file["downloads"]);
            if (url is null)
            {
                skipped++;
                log.Warn($"跳过没有下载地址的文件：{relativePath}");
                continue;
            }

            var target = ResolveInside(gameDir, relativePath);
            if (target is null)
            {
                skipped++;
                log.Warn($"跳过越界路径（可能被 zip 穿越攻击）：{relativePath}");
                continue;
            }

            var directory = Path.GetDirectoryName(target);
            if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
            var sha1 = GetString(file["hashes"]?["sha1"]);
            var size = GetLong(file["fileSize"]);

            tasks.Add(Task.Run(async () =>
            {
                await gate.WaitAsync(ct);
                try
                {
                    await downloader.DownloadAsync(url, target, sha1, size, ct);
                    Interlocked.Increment(ref installed);
                }
                catch (Exception ex) when (ex is DownloadException or HttpRequestException or InvalidOperationException)
                {
                    Interlocked.Increment(ref failed);
                    log.Warn($"下载失败：{relativePath}（{ex.Message}）");
                }
                finally
                {
                    gate.Release();
                    var finished = Interlocked.Increment(ref done);
                    status?.Report($"下载整合包内容 {finished}/{total}");
                }
            }, ct));
        }

        await Task.WhenAll(tasks);
        log.Info($"内容文件处理完成：成功 {installed}，失败 {failed}，跳过 {skipped}（共 {total}）");
        return (installed, failed);
    }

    // ---------------- 覆盖文件 ----------------

    /// <summary>复制 overrides 与 client-overrides（后者优先）到游戏目录，返回复制文件数。</summary>
    private int CopyOverrides(string tempDir, string gameDir)
    {
        var copied = 0;
        foreach (var folder in new[] { "overrides", "client-overrides" })
        {
            var source = Path.Combine(tempDir, folder);
            if (!Directory.Exists(source)) continue;

            foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
            {
                var relativePath = Path.GetRelativePath(source, file);
                var target = ResolveInside(gameDir, relativePath);
                if (target is null)
                {
                    log.Warn($"跳过越界的覆盖文件：{relativePath}");
                    continue;
                }

                var directory = Path.GetDirectoryName(target);
                if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
                File.Copy(file, target, overwrite: true);
                copied++;
            }
        }

        return copied;
    }

    // ---------------- 工具 ----------------

    private JsonObject ReadIndex(string tempDir, string slug)
    {
        var indexPath = Path.Combine(tempDir, IndexFileName);
        if (!File.Exists(indexPath))
            throw new InvalidOperationException($"整合包 {slug} 缺少 {IndexFileName}，不是有效的 .mrpack 文件");

        try
        {
            return JsonNode.Parse(File.ReadAllText(indexPath)) as JsonObject
                   ?? throw new InvalidOperationException($"整合包 {slug} 的 {IndexFileName} 不是 JSON 对象");
        }
        catch (JsonException ex)
        {
            throw new InvalidOperationException($"整合包 {slug} 的 {IndexFileName} 解析失败：{ex.Message}", ex);
        }
    }

    private static string CreateTempDirectory()
    {
        var tempDir = Path.Combine(Path.GetTempPath(), "dlp-mrpack-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(tempDir);
        return tempDir;
    }

    /// <summary>整合包压缩包格式。</summary>
    private enum PackArchiveFormat { Unknown, Modrinth, CurseForge }

    /// <summary>
    /// 只看压缩包目录判断格式（不解压）：根目录有 modrinth.index.json → Modrinth；
    /// 根目录有 manifest.json 且 manifestType 为 minecraftModpack → CurseForge；其余（含打不开的非 zip）→ Unknown。
    /// </summary>
    private static PackArchiveFormat DetectFormat(string path)
    {
        try
        {
            using var archive = ZipFile.OpenRead(path);
            ZipArchiveEntry? manifest = null;
            foreach (var entry in archive.Entries)
            {
                var name = NormalizeEntryPath(entry.FullName);
                if (string.Equals(name, IndexFileName, StringComparison.OrdinalIgnoreCase))
                    return PackArchiveFormat.Modrinth;
                if (string.Equals(name, CurseForgeInstaller.ManifestFileName, StringComparison.OrdinalIgnoreCase))
                    manifest = entry;
            }

            if (manifest is null) return PackArchiveFormat.Unknown;

            using var stream = manifest.Open();
            using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: true);
            using var document = JsonDocument.Parse(reader.ReadToEnd());
            if (document.RootElement.ValueKind != JsonValueKind.Object) return PackArchiveFormat.Unknown;
            if (!document.RootElement.TryGetProperty("manifestType", out var type) || type.ValueKind != JsonValueKind.String)
                return PackArchiveFormat.Unknown;

            return string.Equals(type.GetString(), CurseForgeInstaller.ManifestType, StringComparison.OrdinalIgnoreCase)
                ? PackArchiveFormat.CurseForge
                : PackArchiveFormat.Unknown;
        }
        catch (InvalidDataException)
        {
            return PackArchiveFormat.Unknown; // 不是 zip
        }
        catch (JsonException)
        {
            return PackArchiveFormat.Unknown; // 清单不是合法 JSON，交给后续流程报错
        }
    }

    /// <summary>统一 zip 条目路径：反斜杠转斜杠、去掉开头的 "./"。</summary>
    private static string NormalizeEntryPath(string path)
    {
        var normalized = path.Replace('\\', '/');
        while (normalized.StartsWith("./", StringComparison.Ordinal)) normalized = normalized[2..];
        return normalized;
    }

    /// <summary>由整合包名称生成小写 slug：只保留字母数字，其余折叠成单个 '-'。</summary>
    private static string MakeSlug(string? name)
    {
        var raw = (name ?? string.Empty).ToLowerInvariant();
        var builder = new StringBuilder(raw.Length);
        foreach (var ch in raw)
        {
            if (char.IsAsciiLetterOrDigit(ch))
            {
                builder.Append(ch);
            }
            else if (builder.Length > 0 && builder[^1] != '-')
            {
                builder.Append('-');
            }
        }

        var result = builder.ToString().Trim('-', '.');
        if (result.Length == 0) return "local-pack";
        return result.Length <= 60 ? result : result[..60].TrimEnd('-');
    }

    /// <summary>文件名安全化：只保留文件名本身，替换非法字符。</summary>
    private static string SafeFileName(string fileName)
    {
        var name = Path.GetFileName(fileName);
        if (string.IsNullOrWhiteSpace(name)) name = "modpack.mrpack";
        foreach (var invalid in Path.GetInvalidFileNameChars())
            name = name.Replace(invalid, '_');
        return name;
    }

    /// <summary>版本 id：slug-整合包版本，非字母数字.- 换成 '-'，合并连续 '-'，最长 80。</summary>
    private static string MakeVersionId(string slug, string modpackVersion)
    {
        var raw = $"{slug}-{modpackVersion}";
        var builder = new StringBuilder(raw.Length);
        foreach (var ch in raw)
        {
            var c = char.IsAsciiLetterOrDigit(ch) || ch is '.' or '-' ? ch : '-';
            if (c == '-' && builder.Length > 0 && builder[^1] == '-') continue;
            builder.Append(c);
        }

        var result = builder.ToString().Trim('-', '.');
        if (result.Length == 0) result = "modpack";
        return result.Length <= 80 ? result : result[..80];
    }

    /// <summary>把清单里的相对路径解析到游戏目录下；越界（含绝对路径、.. 上跳）返回 null。</summary>
    private static string? ResolveInside(string root, string relativePath)
    {
        var fullRoot = Path.GetFullPath(root);
        var rootWithSeparator = fullRoot.EndsWith(Path.DirectorySeparatorChar)
            ? fullRoot
            : fullRoot + Path.DirectorySeparatorChar;

        var normalized = relativePath.Replace('/', Path.DirectorySeparatorChar);
        var target = Path.GetFullPath(Path.Combine(fullRoot, normalized));
        return target.StartsWith(rootWithSeparator, StringComparison.OrdinalIgnoreCase) ? target : null;
    }

    private static string? FirstDownloadUrl(JsonNode? node)
    {
        if (node is not JsonArray array) return null;
        foreach (var item in array)
        {
            var url = GetString(item);
            if (!string.IsNullOrWhiteSpace(url)) return url;
        }

        return null;
    }

    private static string? GetString(JsonNode? node)
    {
        if (node is null) return null;
        try { return node.GetValue<string>(); }
        catch (Exception) { return null; }
    }

    private static long? GetLong(JsonNode? node)
    {
        if (node is null) return null;
        try { return node.GetValue<long>(); }
        catch (Exception) { return null; }
    }

    private static void TryDeleteDirectory(string path)
    {
        try { if (Directory.Exists(path)) Directory.Delete(path, recursive: true); }
        catch { /* 临时目录清理失败不影响安装结果 */ }
    }
}
