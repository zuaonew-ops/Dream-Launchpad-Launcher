using System.IO;
using System.Text.Json;
using DLP.Core.Infrastructure;

namespace DLP.App.Services;

public sealed class AppSettings
{
    public int MemoryMb { get; set; } = 2048;
    public string Username { get; set; } = "Steve";
    public string? JavaPath { get; set; }
    public string DownloadSource { get; set; } = "official";

    /// <summary>配置迁移版本；2 表示已把旧的默认镜像源切成官方源。</summary>
    public int SchemaVersion { get; set; }
    public bool VersionIsolation { get; set; } = true;
    public string? MicrosoftClientId { get; set; }
    public string? SelectedAccount { get; set; }

    private static string FilePath => Path.Combine(GamePaths.CreateDefault().Root, "config.json");

    public static AppSettings Load()
    {
        try
        {
            if (File.Exists(FilePath))
            {
                var settings = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(FilePath)) ?? new AppSettings();
                // 旧版本默认用镜像，实测比官方 CDN 慢一倍以上，这里一次性切回官方。
                if (settings.SchemaVersion < 2)
                {
                    settings.DownloadSource = "official";
                    settings.SchemaVersion = 2;
                    settings.Save();
                }
                return settings;
            }
        }
        catch { /* 损坏则回退默认 */ }
        return new AppSettings();
    }

    public void Save()
    {
        try
        {
            var dir = Path.GetDirectoryName(FilePath)!;
            Directory.CreateDirectory(dir);
            var temp = FilePath + ".tmp";
            File.WriteAllText(temp, JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true }));
            File.Move(temp, FilePath, overwrite: true);
        }
        catch { /* 忽略写失败 */ }
    }
}
