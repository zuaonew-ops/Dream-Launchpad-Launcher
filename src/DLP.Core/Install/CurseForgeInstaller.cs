// 作者：zuao
using System.IO.Compression;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using DLP.Core.Download;
using DLP.Core.Infrastructure;

namespace DLP.Core.Install;

/// <summary>CurseForge 整合包安装统计。</summary>
public sealed record CurseForgeInstallResult(
    string VersionId,
    string Name,
    string PackVersion,
    string GameDir,
    int ModsInstalled,
    int ModsFailed,
    int OverridesInstalled);

/// <summary>
/// CurseForge 格式整合包（zip 根目录 manifest.json + overrides/）安装：
/// 读清单 → 装原版与加载器（版本 id 沿用加载器安装出来的 id，如 1.20.1-forge-47.4.20）→
/// 并发下载 files[] 到 versions/&lt;id&gt;/game/mods → 解压 overrides 合并进游戏目录（不删其它文件）。
/// 单个模组失败只记警告并计数，不中断整包安装。
/// </summary>
public sealed class CurseForgeInstaller(
    GamePaths paths,
    DownloadClient downloader,
    GameInstaller installer,
    LoaderService loaders,
    CurseForgeApi api,
    Log log)
{
    /// <summary>整合包根目录的清单文件名。</summary>
    public const string ManifestFileName = "manifest.json";

    /// <summary>清单里的格式标识（manifestType 字段）。</summary>
    public const string ManifestType = "minecraftModpack";

    /// <summary>并发下载数：CDN 单连接很慢，8 路可以把带宽吃满。</summary>
    private const int Concurrency = 4;

    /// <summary>失败明细最多记这么多条到日志，避免刷屏。</summary>
    private const int MaxLoggedFailures = 30;

    private static readonly JsonSerializerOptions IndentedJson = new()
    {
        WriteIndented = true,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    /// <summary>
    /// 安装 CurseForge 整合包 zip。<paramref name="mcVersionHint"/> 只在清单缺少 minecraft.version 时使用。
    /// </summary>
    public async Task<CurseForgeInstallResult> InstallAsync(string zipPath, string? mcVersionHint,
        IProgress<string>? status, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(zipPath);
        var fullPath = Path.GetFullPath(zipPath);
        if (!File.Exists(fullPath)) throw new FileNotFoundException("找不到整合包文件", fullPath);

        status?.Report("读取 CurseForge 整合包清单");
        var pack = ReadManifest(fullPath);

        var mcVersion = string.IsNullOrWhiteSpace(pack.McVersion) ? mcVersionHint : pack.McVersion;
        if (string.IsNullOrWhiteSpace(mcVersion))
            throw new InvalidOperationException("整合包 manifest.json 缺少 minecraft.version，无法确定游戏版本");
        if (!string.IsNullOrWhiteSpace(pack.McVersion) && !string.IsNullOrWhiteSpace(mcVersionHint)
            && !string.Equals(pack.McVersion, mcVersionHint, StringComparison.OrdinalIgnoreCase))
            log.Warn($"整合包声明的游戏版本是 {pack.McVersion}，与命令参数 {mcVersionHint} 不一致，按清单安装");

        var loaderText = pack.LoaderType is null ? "无加载器" : $"加载器 {pack.LoaderType} {pack.LoaderVersion}";
        log.Info($"CurseForge 整合包 {fullPath}：{pack.Name} {pack.Version}，游戏版本 {mcVersion}，{loaderText}，模组 {pack.Files.Count} 个");

        var versionId = await EnsureBaseAsync(mcVersion, pack, status, ct);

        var gameDir = paths.GameDir(versionId);
        Directory.CreateDirectory(gameDir);
        log.Info($"整合包版本 {versionId}，游戏目录 {gameDir}");

        status?.Report("解压覆盖文件");
        var overrides = ExtractOverrides(fullPath, pack.OverridesFolder, gameDir);
        log.Info($"覆盖文件解压完成：{overrides} 个（{pack.OverridesFolder}）");

        var (installed, failed) = await DownloadModsAsync(pack.Files, gameDir, status, ct);

        WritePackMetadata(versionId, pack, mcVersion, installed, failed, overrides);

        if (installed == 0 && failed > 0)
            throw new InvalidOperationException($"CurseForge 整合包 {versionId} 的 {failed} 个模组全部下载失败");
        if (failed > 0)
            log.Warn($"CurseForge 整合包 {versionId}：成功 {installed} 个模组，失败 {failed} 个；重新执行同一命令会续传剩余文件");

        status?.Report($"安装完成 {versionId}");
        log.Info($"安装完成 {versionId}：{installed} 个模组，{failed} 个失败，{overrides} 个覆盖文件");
        return new CurseForgeInstallResult(versionId, pack.Name, pack.Version, gameDir, installed, failed, overrides);
    }

    // ---------------- 底座（原版 + 加载器） ----------------

    private async Task<string> EnsureBaseAsync(string mcVersion, CfPack pack, IProgress<string>? status, CancellationToken ct)
    {
        status?.Report("安装基础版本");
        if (!File.Exists(paths.VersionJson(mcVersion)))
        {
            log.Info($"未检测到原版 {mcVersion}，先安装");
            await installer.InstallAsync(mcVersion, null, ct, asDependency: true);
        }

        if (pack.LoaderType is not { } type || string.IsNullOrWhiteSpace(pack.LoaderVersion))
        {
            // 没有加载器：单独建 <mcVersion>-modpack，避免和纯原版共用同一个游戏目录。
            var id = $"{mcVersion}-modpack";
            WriteVersionJson(id, mcVersion, pack);
            log.Info($"整合包未声明加载器，底座用原版 {mcVersion}，版本 id {id}");
            return id;
        }

        // 已经装过同一个加载器时直接复用，不再跑一遍 Forge/NeoForge 安装器。
        if (FindLoaderVersionId(mcVersion, type, pack.LoaderVersion) is { } existing)
        {
            log.Info($"复用已安装的加载器版本 {existing}");
            return existing;
        }

        status?.Report($"安装加载器 {type} {pack.LoaderVersion}");
        var installed = await loaders.InstallAsync(type, mcVersion, pack.LoaderVersion, status, ct);
        var resolved = FindLoaderVersionId(mcVersion, type, pack.LoaderVersion)
                       ?? (File.Exists(paths.VersionJson(installed)) ? installed : null)
                       ?? throw new InvalidOperationException(
                           $"加载器 {type} {pack.LoaderVersion} 安装完成，但找不到对应的版本描述（返回 {installed}）");
        log.Info($"加载器安装完成：{resolved}");
        return resolved;
    }

    /// <summary>
    /// 加载器安装出来的版本 id 各家不同：Forge 是 &lt;mc&gt;-forge-&lt;ver&gt;，NeoForge 是 &lt;mc&gt;-neoforge-&lt;ver&gt; 或 neoforge-&lt;ver&gt;，
    /// Fabric/Quilt 是 &lt;类型&gt;-loader-&lt;ver&gt;-&lt;mc&gt;。按候选 id 找已经存在的版本描述，找到即复用。
    /// </summary>
    private string? FindLoaderVersionId(string mcVersion, ModLoaderType type, string loaderVersion)
    {
        IEnumerable<string> candidates = type switch
        {
            ModLoaderType.Forge =>
            [
                $"{mcVersion}-forge-{loaderVersion}",
                $"forge-{mcVersion}-{loaderVersion}",
            ],
            ModLoaderType.NeoForge =>
            [
                $"{mcVersion}-neoforge-{loaderVersion}",
                $"neoforge-{loaderVersion}",
                $"neoforge-{mcVersion}-{loaderVersion}",
            ],
            ModLoaderType.Fabric => [$"fabric-loader-{loaderVersion}-{mcVersion}"],
            _ => [$"quilt-loader-{loaderVersion}-{mcVersion}"],
        };

        return candidates.FirstOrDefault(id => File.Exists(paths.VersionJson(id)));
    }

    // ---------------- 清单解析 ----------------

    /// <summary>读 zip 根目录的 manifest.json，解析名称 / 游戏版本 / 加载器 / overrides 目录 / files[]。</summary>
    private CfPack ReadManifest(string zipPath)
    {
        string json;
        try
        {
            using var archive = ZipFile.OpenRead(zipPath);
            var entry = FindEntry(archive, ManifestFileName)
                        ?? throw new InvalidOperationException($"整合包缺少 {ManifestFileName}，不是 CurseForge 格式的整合包");
            using var stream = entry.Open();
            using var reader = new StreamReader(stream, Encoding.UTF8, detectEncodingFromByteOrderMarks: true);
            json = reader.ReadToEnd();
        }
        catch (InvalidDataException ex)
        {
            throw new InvalidOperationException("不是有效的整合包压缩包", ex);
        }

        JsonObject root;
        try
        {
            root = JsonNode.Parse(json) as JsonObject
                   ?? throw new InvalidOperationException($"{ManifestFileName} 不是 JSON 对象");
        }
        catch (JsonException ex)
        {
            throw new InvalidOperationException($"{ManifestFileName} 解析失败：{ex.Message}", ex);
        }

        var manifestType = GetString(root["manifestType"]);
        if (!string.IsNullOrWhiteSpace(manifestType) && !string.Equals(manifestType, ManifestType, StringComparison.OrdinalIgnoreCase))
            log.Warn($"manifestType 是 {manifestType}（不是 {ManifestType}），仍按 CurseForge 整合包处理");

        var name = GetString(root["name"]);
        if (string.IsNullOrWhiteSpace(name)) name = Path.GetFileNameWithoutExtension(zipPath);
        var version = GetString(root["version"]);
        if (string.IsNullOrWhiteSpace(version)) version = "1.0.0";

        var minecraft = root["minecraft"] as JsonObject;
        var mcVersion = GetString(minecraft?["version"]);
        var (loaderType, loaderVersion) = ParseLoader(minecraft?["modLoaders"], mcVersion);

        var overrides = GetString(root["overrides"]);
        if (string.IsNullOrWhiteSpace(overrides)) overrides = "overrides";

        var files = new List<CfFile>();
        var optional = 0;
        var malformed = 0;
        if (root["files"] is JsonArray array)
        {
            foreach (var item in array)
            {
                if (item is not JsonObject file)
                {
                    malformed++;
                    continue;
                }

                var projectId = GetInt(file["projectID"]);
                var fileId = GetInt(file["fileID"]);
                if (projectId is null || fileId is null)
                {
                    malformed++;
                    continue;
                }

                if (GetBool(file["required"]) == false)
                {
                    optional++;
                    continue;
                }

                files.Add(new CfFile(projectId.Value, fileId.Value));
            }
        }

        if (malformed > 0) log.Warn($"清单里有 {malformed} 个条目缺少 projectID/fileID 或格式不对，已跳过");
        if (optional > 0) log.Info($"清单里有 {optional} 个 required=false 的可选文件，跳过");
        if (files.Count == 0)
            throw new InvalidOperationException("CurseForge 整合包的 files[] 为空，没有可安装的模组");

        return new CfPack(name!, version!, mcVersion, loaderType, loaderVersion, overrides, files);
    }

    /// <summary>解析 minecraft.modLoaders：优先 primary=true 的一项，id 形如 forge-47.4.20 / fabric-0.15.11 / neoforge-20.4.237。</summary>
    private static (ModLoaderType? Type, string? Version) ParseLoader(JsonNode? node, string? mcVersion)
    {
        if (node is not JsonArray array || array.Count == 0) return (null, null);

        string? id = null;
        foreach (var item in array)
        {
            if (item is not JsonObject loader) continue;
            var value = GetString(loader["id"]);
            if (string.IsNullOrWhiteSpace(value)) continue;
            if (GetBool(loader["primary"]) == true)
            {
                id = value;
                break;
            }

            id ??= value;
        }

        if (string.IsNullOrWhiteSpace(id)) return (null, null);

        var text = id.Trim();
        var dash = text.IndexOf('-');
        if (dash <= 0) return (null, null);

        var prefix = text[..dash].ToLowerInvariant();
        var version = text[(dash + 1)..].Trim();

        var type = prefix switch
        {
            "forge" => ModLoaderType.Forge,
            "neoforge" => ModLoaderType.NeoForge,
            "fabric" => ModLoaderType.Fabric,
            "quilt" => ModLoaderType.Quilt,
            _ => (ModLoaderType?)null,
        };

        // Fabric 的 id 偶尔写成 fabric-0.15.11-1.20.1，去掉尾部原版版本号。
        if (type is ModLoaderType.Fabric or ModLoaderType.Quilt && !string.IsNullOrWhiteSpace(mcVersion)
            && version.EndsWith("-" + mcVersion, StringComparison.OrdinalIgnoreCase))
            version = version[..^(mcVersion.Length + 1)];

        if (type is null || version.Length == 0) return (null, null);
        return (type, version);
    }

    // ---------------- 模组下载 ----------------

    private async Task<(int Installed, int Failed)> DownloadModsAsync(IReadOnlyList<CfFile> files, string gameDir,
        IProgress<string>? status, CancellationToken ct)
    {
        var modsDir = Path.Combine(gameDir, "mods");
        Directory.CreateDirectory(modsDir);

        var total = files.Count;
        var installed = 0;
        var failed = 0;
        var done = 0;
        var failures = new List<string>();

        using var gate = new SemaphoreSlim(Concurrency);
        var tasks = new List<Task>(total);

        foreach (var file in files)
        {
            ct.ThrowIfCancellationRequested();
            tasks.Add(Task.Run(async () =>
            {
                await gate.WaitAsync(ct);
                string? fileName = null;
                try
                {
                    var meta = await api.ResolveAsync(file.ProjectId, file.FileId, ct)
                               ?? throw new CurseForgeException(
                                   $"cfwidget 只保留最近的文件列表，这个旧文件查不到文件名（projectID {file.ProjectId} / fileID {file.FileId}），已跳过；需要的话去 CurseForge 页面手动下载");

                    fileName = SafeFileName(meta.FileName);
                    var target = Path.Combine(modsDir, fileName);

                    try
                    {
                        // CDN 上的模组文件都很小，单连接就够快；分段反而容易在慢节点上超时。
                        await downloader.DownloadAsync(meta.DownloadUrl, target, null, meta.FileSize, ct, allowRange: false);
                    }
                    catch (DownloadException) when (!string.Equals(meta.DownloadUrl, meta.AlternativeDownloadUrl, StringComparison.Ordinal))
                    {
                        // 主域名失败时换 forgecdn 备用域名（同一路径）。
                        log.Warn($"主 CDN 下载失败，改用备用域名：{fileName}");
                        await downloader.DownloadAsync(meta.AlternativeDownloadUrl, target, null, meta.FileSize, ct, allowRange: false);
                    }

                    Interlocked.Increment(ref installed);
                }
                catch (Exception ex) when (ex is not OperationCanceledException || !ct.IsCancellationRequested)
                {
                    Interlocked.Increment(ref failed);
                    var label = fileName ?? $"projectID {file.ProjectId}/fileID {file.FileId}";
                    lock (failures)
                        if (failures.Count < MaxLoggedFailures) failures.Add($"{label}：{ex.Message}");
                    log.Warn($"模组下载失败 {label}（projectID {file.ProjectId}，fileID {file.FileId}）：{ex.Message}");
                }
                finally
                {
                    gate.Release();
                    var finished = Interlocked.Increment(ref done);
                    status?.Report($"下载模组 {finished}/{total}（失败 {Volatile.Read(ref failed)}）");
                }
            }, ct));
        }

        await Task.WhenAll(tasks);

        log.Info($"模组下载完成：成功 {installed}，失败 {failed}（共 {total}）");
        if (failures.Count > 0)
        {
            var more = failed > failures.Count ? $"，另有 {failed - failures.Count} 个未列出" : string.Empty;
            log.Warn($"失败明细（{failures.Count} 个{more}）：{string.Join("；", failures)}");
        }

        return (installed, failed);
    }

    // ---------------- overrides 解压 ----------------

    /// <summary>把 zip 里的 overrides/&lt;manifest.overrides&gt; 目录逐个解压到游戏目录，同名文件覆盖，不动其它文件。</summary>
    private int ExtractOverrides(string zipPath, string overridesFolder, string gameDir)
    {
        if (string.IsNullOrWhiteSpace(overridesFolder)) return 0;

        var prefix = NormalizeEntryPath(overridesFolder).TrimEnd('/') + "/";
        var copied = 0;

        using var archive = ZipFile.OpenRead(zipPath);
        foreach (var entry in archive.Entries)
        {
            var fullName = NormalizeEntryPath(entry.FullName);
            if (!fullName.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) continue;
            if (fullName.EndsWith('/')) continue; // 目录条目

            var relativePath = fullName[prefix.Length..];
            if (relativePath.Length == 0) continue;

            var target = ResolveInside(gameDir, relativePath);
            if (target is null)
            {
                log.Warn($"跳过越界的覆盖文件：{relativePath}");
                continue;
            }

            var directory = Path.GetDirectoryName(target);
            if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);
            entry.ExtractToFile(target, overwrite: true);
            copied++;
        }

        return copied;
    }

    // ---------------- 版本描述 / 元数据 ----------------

    /// <summary>无加载器的整合包：写一个继承原版的版本描述（已存在则不覆盖）。</summary>
    private void WriteVersionJson(string versionId, string mcVersion, CfPack pack)
    {
        var target = paths.VersionJson(versionId);
        if (File.Exists(target)) return;

        Directory.CreateDirectory(paths.VersionDir(versionId));
        var now = DateTimeOffset.Now.ToString("o");
        var json = new JsonObject
        {
            ["id"] = versionId,
            ["type"] = "release",
            ["inheritsFrom"] = mcVersion,
            ["time"] = now,
            ["releaseTime"] = now,
            ["x-dlp-modpack"] = new JsonObject
            {
                ["name"] = pack.Name,
                ["slug"] = "curseforge",
                ["source"] = "curseforge",
            },
        };
        File.WriteAllText(target, json.ToJsonString(IndentedJson));
        log.Info($"写入版本描述：{target}");
    }

    /// <summary>在版本目录里记一份整合包来源信息（不覆盖加载器自己的版本 json）。</summary>
    private void WritePackMetadata(string versionId, CfPack pack, string mcVersion, int installed, int failed, int overrides)
    {
        var json = new JsonObject
        {
            ["source"] = "curseforge",
            ["name"] = pack.Name,
            ["version"] = pack.Version,
            ["minecraft"] = mcVersion,
            ["loader"] = pack.LoaderType is { } loaderType ? $"{loaderType.ToString().ToLowerInvariant()}-{pack.LoaderVersion}" : null,
            ["modsDeclared"] = pack.Files.Count,
            ["modsInstalled"] = installed,
            ["modsFailed"] = failed,
            ["overridesInstalled"] = overrides,
            ["installedAt"] = DateTimeOffset.Now.ToString("o"),
        };

        var target = Path.Combine(paths.VersionDir(versionId), "curseforge-pack.json");
        File.WriteAllText(target, json.ToJsonString(IndentedJson));
        log.Info($"写入整合包元数据：{target}");
    }

    // ---------------- 工具 ----------------

    private static ZipArchiveEntry? FindEntry(ZipArchive archive, string fileName)
        => archive.Entries.FirstOrDefault(entry =>
            string.Equals(NormalizeEntryPath(entry.FullName), fileName, StringComparison.OrdinalIgnoreCase));

    /// <summary>统一 zip 条目路径：反斜杠转斜杠、去掉开头的 "./"。</summary>
    private static string NormalizeEntryPath(string path)
    {
        var normalized = path.Replace('\\', '/');
        while (normalized.StartsWith("./", StringComparison.Ordinal)) normalized = normalized[2..];
        return normalized;
    }

    /// <summary>只留文件名本身（防止清单里带目录穿越），并替换 Windows 非法字符。</summary>
    private static string SafeFileName(string name)
    {
        var bare = Path.GetFileName(name.Replace('\\', Path.DirectorySeparatorChar));
        if (string.IsNullOrWhiteSpace(bare)) bare = name;

        var builder = new StringBuilder(bare.Length);
        foreach (var ch in bare)
            builder.Append(Array.IndexOf(Path.GetInvalidFileNameChars(), ch) >= 0 ? '_' : ch);

        var result = builder.ToString().Trim();
        return result.Length == 0 ? "mod.jar" : result;
    }

    /// <summary>把相对路径解析到游戏目录下；越界（绝对路径、.. 上跳）返回 null。</summary>
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

    private static string? GetString(JsonNode? node)
    {
        if (node is null) return null;
        try { return node.GetValue<string>(); }
        catch (Exception) { return null; }
    }

    private static int? GetInt(JsonNode? node)
    {
        if (node is null) return null;
        try { return node.GetValue<int>(); }
        catch (Exception) { return null; }
    }

    private static bool? GetBool(JsonNode? node)
    {
        if (node is null) return null;
        try { return node.GetValue<bool>(); }
        catch (Exception) { return null; }
    }

    /// <summary>清单里的一条文件依赖。</summary>
    private sealed record CfFile(int ProjectId, int FileId);

    /// <summary>解析后的 CurseForge 清单。</summary>
    private sealed record CfPack(
        string Name,
        string Version,
        string? McVersion,
        ModLoaderType? LoaderType,
        string? LoaderVersion,
        string OverridesFolder,
        IReadOnlyList<CfFile> Files);
}
