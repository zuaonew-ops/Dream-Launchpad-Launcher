namespace DLP.Core.Infrastructure;

/// <summary>数据目录布局：始终放在启动器 exe 所在文件夹（便携），只管理这个文件夹里的内容。</summary>
public sealed class GamePaths
{
    public string Root { get; }
    public string Versions => Path.Combine(Root, "versions");
    public string Libraries => Path.Combine(Root, "libraries");
    public string Assets => Path.Combine(Root, "assets");
    public string AssetIndexes => Path.Combine(Assets, "indexes");
    public string AssetObjects => Path.Combine(Assets, "objects");
    public string Runtime => Path.Combine(Root, "runtime");
    public string Logs => Path.Combine(Root, "logs");
    public string Cache => Path.Combine(Root, "cache");

    public GamePaths(string root)
    {
        Root = root;
        Directory.CreateDirectory(Root);
        foreach (var dir in new[] { Versions, Libraries, Assets, AssetIndexes, AssetObjects, Runtime, Logs, Cache })
            Directory.CreateDirectory(dir);
    }

    /// <summary>便携模式：数据目录固定为 exe 所在目录。</summary>
    public static GamePaths CreateDefault() => new(AppContext.BaseDirectory);

    public string VersionDir(string id) => Path.Combine(Versions, id);
    public string VersionJson(string id) => Path.Combine(VersionDir(id), id + ".json");
    public string VersionJar(string id) => Path.Combine(VersionDir(id), id + ".jar");
    public string NativesDir(string id) => Path.Combine(VersionDir(id), "natives");

    /// <summary>版本隔离：每个版本独立游戏目录 versions/&lt;id&gt;/game。</summary>
    public string GameDir(string versionId) => Path.Combine(VersionDir(versionId), "game");
    public string GameSubDir(string versionId, string name) => Path.Combine(GameDir(versionId), name);
}
