using System.Diagnostics;
using System.Text.Json;
using System.Text.RegularExpressions;
using DLP.Core.Download;
using DLP.Core.Infrastructure;
using DLP.Core.Minecraft;

namespace DLP.Core.Java;

public sealed record JavaRuntime(string Path, int MajorVersion, bool Is64Bit, string RawVersion, string Source);

/// <summary>Java 扫描 / 选择 / Mojang JRE 下载。</summary>
public sealed class JavaManager(GamePaths paths, HttpClient http, DownloadClient downloader, Log log)
{
    private const string RuntimeManifestUrl =
        "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

    public IReadOnlyList<JavaRuntime> Scan()
    {
        var candidates = new List<string>();

        if (Environment.GetEnvironmentVariable("JAVA_HOME") is { Length: > 0 } javaHome)
            candidates.Add(Path.Combine(javaHome, "bin", "java.exe"));

        var pathVar = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var dir in pathVar.Split(Path.PathSeparator))
        {
            if (string.IsNullOrWhiteSpace(dir)) continue;
            var exe = Path.Combine(dir, "java.exe");
            if (File.Exists(exe)) candidates.Add(exe);
        }

        foreach (var baseDir in new[]
                 {
                     @"C:\Program Files\Java",
                     @"C:\Program Files\Eclipse Adoptium",
                     @"C:\Program Files\Microsoft",
                     @"C:\Program Files\Zulu",
                     @"C:\Program Files\BellSoft",
                     Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Eclipse Adoptium")
                 })
        {
            if (!Directory.Exists(baseDir)) continue;
            try
            {
                foreach (var dir in Directory.EnumerateDirectories(baseDir))
                {
                    var exe = Path.Combine(dir, "bin", "java.exe");
                    if (File.Exists(exe)) candidates.Add(exe);
                }
            }
            catch (Exception ex) { log.Debug($"扫描 {baseDir} 失败：{ex.Message}"); }
        }

        foreach (var exe in Directory.EnumerateFiles(paths.Runtime, "java.exe", SearchOption.AllDirectories))
            candidates.Add(exe);

        return candidates
            .Select(Path.GetFullPath)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(Probe)
            .OfType<JavaRuntime>()
            .OrderByDescending(j => j.MajorVersion)
            .ToList();
    }

    private JavaRuntime? Probe(string path)
    {
        try
        {
            var home = Path.GetFullPath(Path.Combine(Path.GetDirectoryName(path)!, ".."));
            var releaseFile = Path.Combine(home, "release");
            if (File.Exists(releaseFile))
            {
                var text = File.ReadAllText(releaseFile);
                var version = Regex.Match(text, "JAVA_VERSION=\"([^\"]+)\"").Groups[1].Value;
                var arch = Regex.Match(text, "OS_ARCH=\"([^\"]+)\"").Groups[1].Value;
                if (version.Length > 0)
                    return new JavaRuntime(path, ParseMajor(version), arch.Contains("64"), version, home);
            }

            var psi = new ProcessStartInfo(path, "-XshowSettings:properties -version")
            {
                RedirectStandardError = true,
                RedirectStandardOutput = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var process = Process.Start(psi);
            if (process is null) return null;
            var output = process.StandardError.ReadToEnd() + process.StandardOutput.ReadToEnd();
            process.WaitForExit(5000);
            var match = Regex.Match(output, "java.version = (\\S+)");
            if (!match.Success) return null;
            return new JavaRuntime(path, ParseMajor(match.Groups[1].Value), output.Contains("os.arch = amd64") || output.Contains("os.arch = x86_64"), match.Groups[1].Value, home);
        }
        catch (Exception ex)
        {
            log.Debug($"探测 Java 失败 {path}：{ex.Message}");
            return null;
        }
    }

    public static int ParseMajor(string version)
    {
        if (version.StartsWith("1.")) return int.TryParse(version.Split('.')[1], out var v) ? v : 8;
        return int.TryParse(version.Split('.')[0], out var major) ? major : 0;
    }

    /// <summary>选择最合适的 Java：优先精确匹配要求的主版本，其次大于要求的最低版本。</summary>
    public JavaRuntime? Select(int requiredMajor)
    {
        var runtimes = Scan();
        return runtimes.FirstOrDefault(j => j.MajorVersion == requiredMajor && j.Is64Bit)
               ?? runtimes.FirstOrDefault(j => j.MajorVersion == requiredMajor)
               ?? runtimes.Where(j => j.MajorVersion >= requiredMajor && j.Is64Bit).OrderBy(j => j.MajorVersion).FirstOrDefault()
               ?? runtimes.Where(j => j.MajorVersion >= requiredMajor).OrderBy(j => j.MajorVersion).FirstOrDefault();
    }

    public async Task<JavaRuntime> EnsureAsync(VersionJson version, CancellationToken ct = default)
    {
        var required = version.JavaVersion?.MajorVersion ?? 8;
        if (Select(required) is { } existing) return existing;

        var component = version.JavaVersion?.Component;
        if (string.IsNullOrEmpty(component)) component = required switch
        {
            <= 8 => "java-runtime-alpha",
            11 => "java-runtime-beta",
            17 => "java-runtime-gamma",
            21 => "java-runtime-delta",
            _ => "java-runtime-epsilon"
        };

        log.Info($"未找到 Java {required}，下载 Mojang 运行时 {component}");
        var javaPath = await DownloadRuntimeAsync(component, ct);
        return new JavaRuntime(javaPath, required, true, required.ToString(), Path.GetDirectoryName(javaPath)!);
    }

    private async Task<string> DownloadRuntimeAsync(string component, CancellationToken ct)
    {
        var allJson = await http.GetStringAsync(RuntimeManifestUrl, ct);
        using var doc = JsonDocument.Parse(allJson);
        var platform = doc.RootElement.GetProperty("windows-x64");
        if (!platform.TryGetProperty(component, out var entries))
            throw new InvalidOperationException($"Mojang 未提供运行时组件：{component}");

        var manifestUrl = entries[0].GetProperty("manifest").GetProperty("url").GetString()
                          ?? throw new InvalidOperationException("运行时清单缺少 URL");
        var manifestJson = await http.GetStringAsync(manifestUrl, ct);
        var manifest = JsonSerializer.Deserialize<JavaRuntimeManifest>(manifestJson)
                       ?? throw new InvalidOperationException("运行时清单无效");

        var root = Path.Combine(paths.Runtime, component, "windows-x64");
        Directory.CreateDirectory(root);

        using var gate = new SemaphoreSlim(8);
        var jobs = new List<Task>();
        foreach (var (relative, file) in manifest.Files)
        {
            var target = Path.GetFullPath(Path.Combine(root, relative));
            if (!target.StartsWith(Path.GetFullPath(root) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                continue;

            if (file.Type == "directory")
            {
                Directory.CreateDirectory(target);
                continue;
            }
            if (file.Downloads?.Raw is not { } raw || string.IsNullOrEmpty(raw.Url)) continue;

            await gate.WaitAsync(ct);
            jobs.Add(Task.Run(async () =>
            {
                try { await downloader.DownloadAsync(raw.Url, target, raw.Sha1, raw.Size, ct); }
                finally { gate.Release(); }
            }, ct));
        }
        await Task.WhenAll(jobs);

        var javaExe = Path.Combine(root, "bin", "java.exe");
        if (!File.Exists(javaExe)) throw new InvalidOperationException($"运行时下载后未找到 java.exe：{javaExe}");
        return javaExe;
    }
}