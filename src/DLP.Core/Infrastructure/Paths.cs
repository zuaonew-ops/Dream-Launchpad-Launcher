namespace DLP.Core.Infrastructure;

/// <summary>启动器数据目录布局。root 默认 %APPDATA%\DreamLaunchpad，便携模式下为 exe 目录。</summary>
public sealed class GamePaths
{
    public string Root { get; }
    public string Versions => Path.Combine(Root, "versions");
    public string Libraries => Path.Combine(Root, "libraries");
    public string Assets => Path.Combine(Root, "assets");
    public string AssetIndexes => Path.Combine(Assets, "indexes");
    public string AssetObjects => Path.Combine(Assets, "objects");
    public string Runtime => Path.Combine(Root, "runtime");
    public string Instances => Path.Combine(Root, "instances");
    public string Logs => Path.Combine(Root, "logs");
    public string Cache => Path.Combine(Root, "cache");

    public GamePaths(string root)
    {
        Root = root;
        Directory.CreateDirectory(Root);
        foreach (var dir in new[] { Versions, Libraries, Assets, AssetIndexes, AssetObjects, Runtime, Instances, Logs, Cache })
            Directory.CreateDirectory(dir);
    }

    public static GamePaths CreateDefault()
    {
        var exeDir = AppContext.BaseDirectory;
        if (File.Exists(Path.Combine(exeDir, "portable.txt")))
            return new GamePaths(Path.Combine(exeDir, ".dlp"));
        return new GamePaths(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "DreamLaunchpad"));
    }

    public string VersionDir(string id) => Path.Combine(Versions, id);
    public string VersionJson(string id) => Path.Combine(VersionDir(id), id + ".json");
    public string VersionJar(string id) => Path.Combine(VersionDir(id), id + ".jar");
    public string NativesDir(string id) => Path.Combine(VersionDir(id), "natives");
    public string InstanceDir(string name) => Path.Combine(Instances, name);
    public string InstanceGameDir(string name) => Path.Combine(InstanceDir(name), "minecraft");
}
