using System.Diagnostics;
using System.Text;
using DLP.Core.Auth;
using DLP.Core.Infrastructure;
using DLP.Core.Minecraft;

namespace DLP.Core.Launch;

public sealed class LaunchOptions
{
    public required LaunchAccount Account { get; init; }
    public required string JavaPath { get; init; }
    public required string GameDirectory { get; init; }
    public int MemoryMb { get; init; } = 2048;
    public List<string> ExtraJvmArgs { get; init; } = [];
    public List<string> ExtraGameArgs { get; init; } = [];
    public string LauncherName { get; init; } = "DLP";
    public string LauncherVersion { get; init; } = "0.1.0";
}

/// <summary>构建启动参数并启动游戏进程。</summary>
public sealed class GameLauncher(GamePaths paths, Log log)
{
    public Process BuildProcess(VersionJson version, LaunchOptions options)
    {
        var nativesDir = paths.NativesDir(version.Id);
        var classpath = BuildClasspath(version);
        var variables = BuildVariables(version, options, nativesDir, classpath);

        var args = new List<string>();
        args.Add($"-Xms{Math.Max(512, options.MemoryMb / 4)}M");
        args.Add($"-Xmx{options.MemoryMb}M");

        var jvmArgs = version.Arguments?.Jvm ?? [];
        if (jvmArgs.Count > 0)
        {
            var expanded = ExpandArguments(jvmArgs, variables);
            args.AddRange(expanded);
            if (!expanded.Contains("-cp") && !expanded.Contains("-classpath")) args.AddRange(["-cp", classpath]);
        }
        else
        {
            args.AddRange(["-Djava.library.path=" + nativesDir, "-cp", classpath]);
        }

        args.AddRange(options.ExtraJvmArgs);

        var gameArgs = version.Arguments?.Game is { Count: > 0 } modern
            ? ExpandArguments(modern, variables)
            : ExpandLegacy(version.MinecraftArguments, variables);
        gameArgs.AddRange(options.ExtraGameArgs);

        args.Add(version.MainClass);
        args.AddRange(gameArgs);

        var latestLog = Path.Combine(options.GameDirectory, "logs", "latest.log");
        Directory.CreateDirectory(Path.GetDirectoryName(latestLog)!);
        File.WriteAllText(latestLog, $"# DLP 启动 {DateTime.Now:yyyy-MM-dd HH:mm:ss}\n# 版本 {version.Id}\n");

        log.Info($"Java: {options.JavaPath}");
        log.Info($"命令行: {options.JavaPath} {string.Join(' ', args.Select(QuoteForLog))}");

        var startInfo = new ProcessStartInfo(options.JavaPath)
        {
            WorkingDirectory = options.GameDirectory,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
            StandardOutputEncoding = Encoding.UTF8,
            StandardErrorEncoding = Encoding.UTF8
        };
        foreach (var arg in args) startInfo.ArgumentList.Add(arg);

        var process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };
        process.OutputDataReceived += (_, e) => Forward(e.Data, latestLog);
        process.ErrorDataReceived += (_, e) => Forward(e.Data, latestLog);
        return process;
    }

    private void Forward(string? line, string latestLog)
    {
        if (line is null) return;
        log.Write(LogLevel.Info, "[游戏] " + line);
        try { File.AppendAllText(latestLog, line + Environment.NewLine); } catch { /* 忽略 */ }
    }

    private static string QuoteForLog(string arg) => arg.Contains(' ') ? $"\"{arg}\"" : arg;

    public string BuildClasspath(VersionJson version)
    {
        var entries = new List<string>();
        foreach (var library in version.Libraries)
        {
            if (!RuleEvaluator.IsAllowed(library.Rules, OsContext.Windows)) continue;
            if (library.Natives is not null) continue;
            if (library.Downloads?.Artifact is not { } artifact) continue;
            var path = Path.Combine(paths.Libraries, artifact.Path ?? MavenCoordinate.Parse(library.Name).ToPath());
            entries.Add(path);
        }
        entries.Add(paths.VersionJar(version.Id));
        return string.Join(Path.PathSeparator, entries);
    }

    private Dictionary<string, string> BuildVariables(VersionJson version, LaunchOptions options, string nativesDir, string classpath)
    {
        var account = options.Account;
        return new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["auth_player_name"] = account.Username,
            ["auth_uuid"] = account.Uuid.Replace("-", ""),
            ["auth_access_token"] = account.AccessToken,
            ["auth_session"] = $"token:{account.AccessToken}:{account.Uuid}",
            ["user_type"] = account.UserType,
            ["user_properties"] = "{}",
            ["clientid"] = "",
            ["auth_xuid"] = "",
            ["version_name"] = version.Id,
            ["version_type"] = version.Type,
            ["game_directory"] = options.GameDirectory,
            ["game_assets"] = Path.Combine(paths.Assets, "virtual", "legacy"),
            ["assets_root"] = paths.Assets,
            ["assets_index_name"] = version.AssetIndex?.Id ?? version.Assets ?? "legacy",
            ["natives_directory"] = nativesDir,
            ["classpath"] = classpath,
            ["classpath_separator"] = Path.PathSeparator.ToString(),
            ["library_directory"] = paths.Libraries,
            ["launcher_name"] = options.LauncherName,
            ["launcher_version"] = options.LauncherVersion
        };
    }

    private static List<string> ExpandArguments(List<Argument> arguments, Dictionary<string, string> variables)
    {
        var result = new List<string>();
        foreach (var argument in arguments)
        {
            if (!RuleEvaluator.IsAllowed(argument.Rules, OsContext.Windows)) continue;
            foreach (var value in argument.Values) result.Add(Replace(value, variables));
        }
        return result;
    }

    private static List<string> ExpandLegacy(string? minecraftArguments, Dictionary<string, string> variables)
    {
        if (string.IsNullOrWhiteSpace(minecraftArguments)) return [];
        return minecraftArguments
            .Split(' ', StringSplitOptions.RemoveEmptyEntries)
            .Select(token => Replace(token, variables))
            .ToList();
    }

    private static string Replace(string template, Dictionary<string, string> variables)
    {
        var builder = new StringBuilder(template);
        foreach (var (key, value) in variables)
            builder.Replace("${" + key + "}", value);
        return builder.ToString();
    }
}
