namespace DLP.Core.Minecraft;

/// <summary>Maven 坐标 group:artifact:version[:classifier][@ext] 的解析与路径生成。</summary>
public sealed record MavenCoordinate(string Group, string Artifact, string Version, string? Classifier, string Extension)
{
    public string ToPath()
    {
        var file = Classifier is null
            ? $"{Artifact}-{Version}.{Extension}"
            : $"{Artifact}-{Version}-{Classifier}.{Extension}";
        return string.Join('/', Group.Split('.').Append(Artifact).Append(Version).Append(file));
    }

    public static MavenCoordinate Parse(string name)
    {
        var ext = "jar";
        var at = name.IndexOf('@');
        if (at >= 0)
        {
            ext = name[(at + 1)..];
            name = name[..at];
        }

        var parts = name.Split(':');
        if (parts.Length < 3) throw new FormatException($"无效的 Maven 坐标：{name}");
        var classifier = parts.Length >= 4 ? parts[3] : null;
        return new MavenCoordinate(parts[0], parts[1], parts[2], classifier, ext);
    }
}
