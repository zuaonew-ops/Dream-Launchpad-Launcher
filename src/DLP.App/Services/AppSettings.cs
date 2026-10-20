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
    public string LastInstance { get; set; } = "default";

    private static string FilePath => Path.Combine(GamePaths.CreateDefault().Root, "config.json");

    public static AppSettings Load()
    {
        try
        {
            if (File.Exists(FilePath))
                return JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(FilePath)) ?? new AppSettings();
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