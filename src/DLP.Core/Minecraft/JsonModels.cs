using System.Text.Json;
using System.Text.Json.Serialization;

namespace DLP.Core.Minecraft;

public sealed class VersionManifest
{
    [JsonPropertyName("latest")] public LatestVersion Latest { get; set; } = new();
    [JsonPropertyName("versions")] public List<VersionEntry> Versions { get; set; } = [];
}

public sealed class LatestVersion
{
    [JsonPropertyName("release")] public string Release { get; set; } = "";
    [JsonPropertyName("snapshot")] public string Snapshot { get; set; } = "";
}

public sealed class VersionEntry
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("type")] public string Type { get; set; } = "";
    [JsonPropertyName("url")] public string Url { get; set; } = "";
    [JsonPropertyName("sha1")] public string Sha1 { get; set; } = "";
    [JsonPropertyName("releaseTime")] public DateTimeOffset ReleaseTime { get; set; }
    [JsonPropertyName("complianceLevel")] public int ComplianceLevel { get; set; }
}

public sealed class VersionJson
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("inheritsFrom")] public string? InheritsFrom { get; set; }
    [JsonPropertyName("type")] public string Type { get; set; } = "release";
    [JsonPropertyName("mainClass")] public string MainClass { get; set; } = "";
    [JsonPropertyName("assets")] public string? Assets { get; set; }
    [JsonPropertyName("assetIndex")] public AssetIndexRef? AssetIndex { get; set; }
    [JsonPropertyName("downloads")] public VersionDownloads Downloads { get; set; } = new();
    [JsonPropertyName("libraries")] public List<Library> Libraries { get; set; } = [];
    [JsonPropertyName("arguments")] public VersionArguments? Arguments { get; set; }
    [JsonPropertyName("minecraftArguments")] public string? MinecraftArguments { get; set; }
    [JsonPropertyName("javaVersion")] public JavaVersion? JavaVersion { get; set; }
    [JsonPropertyName("logging")] public LoggingConfig? Logging { get; set; }
    [JsonPropertyName("minimumLauncherVersion")] public int? MinimumLauncherVersion { get; set; }
    [JsonPropertyName("releaseTime"), JsonConverter(typeof(LenientDateTimeOffsetConverter))] public DateTimeOffset? ReleaseTime { get; set; }
    [JsonPropertyName("time"), JsonConverter(typeof(LenientDateTimeOffsetConverter))] public DateTimeOffset? Time { get; set; }
}

public sealed class VersionDownloads
{
    [JsonPropertyName("client")] public Artifact? Client { get; set; }
    [JsonPropertyName("server")] public Artifact? Server { get; set; }
}

public sealed class Artifact
{
    [JsonPropertyName("path")] public string? Path { get; set; }
    [JsonPropertyName("sha1")] public string Sha1 { get; set; } = "";
    [JsonPropertyName("size")] public long Size { get; set; }
    [JsonPropertyName("url")] public string Url { get; set; } = "";
}

public sealed class AssetIndexRef
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("sha1")] public string Sha1 { get; set; } = "";
    [JsonPropertyName("size")] public long Size { get; set; }
    [JsonPropertyName("totalSize")] public long TotalSize { get; set; }
    [JsonPropertyName("url")] public string Url { get; set; } = "";
}

public sealed class Library
{
    [JsonPropertyName("name")] public string Name { get; set; } = "";
    [JsonPropertyName("downloads")] public LibraryDownloads? Downloads { get; set; }
    [JsonPropertyName("natives")] public Dictionary<string, string>? Natives { get; set; }
    [JsonPropertyName("rules")] public List<Rule>? Rules { get; set; }
    [JsonPropertyName("extract")] public ExtractRule? Extract { get; set; }
    /// <summary>Fabric/Quilt profile 里没有 downloads，只有 maven 仓库地址。</summary>
    [JsonPropertyName("url")] public string? Url { get; set; }
}

public sealed class LibraryDownloads
{
    [JsonPropertyName("artifact")] public Artifact? Artifact { get; set; }
    [JsonPropertyName("classifiers")] public Dictionary<string, Artifact>? Classifiers { get; set; }
}

public sealed class ExtractRule
{
    [JsonPropertyName("exclude")] public List<string> Exclude { get; set; } = [];
}

public sealed class Rule
{
    [JsonPropertyName("action")] public string Action { get; set; } = "allow";
    [JsonPropertyName("os")] public OsRule? Os { get; set; }
    [JsonPropertyName("features")] public Dictionary<string, bool>? Features { get; set; }
}

public sealed class OsRule
{
    [JsonPropertyName("name")] public string? Name { get; set; }
    [JsonPropertyName("arch")] public string? Arch { get; set; }
    [JsonPropertyName("version")] public string? Version { get; set; }
}

public sealed class VersionArguments
{
    [JsonPropertyName("game")] public List<Argument> Game { get; set; } = [];
    [JsonPropertyName("jvm")] public List<Argument> Jvm { get; set; } = [];
}

[JsonConverter(typeof(ArgumentConverter))]
public sealed class Argument
{
    public List<Rule>? Rules { get; set; }
    public List<string> Values { get; set; } = [];

    public static Argument FromJson(JsonElement element)
    {
        var arg = new Argument();
        if (element.ValueKind == JsonValueKind.String)
        {
            arg.Values.Add(element.GetString() ?? "");
            return arg;
        }
        if (element.TryGetProperty("rules", out var rules) && rules.ValueKind == JsonValueKind.Array)
        {
            arg.Rules = JsonSerializer.Deserialize<List<Rule>>(rules.GetRawText());
        }
        if (element.TryGetProperty("value", out var value))
        {
            if (value.ValueKind == JsonValueKind.String) arg.Values.Add(value.GetString() ?? "");
            else if (value.ValueKind == JsonValueKind.Array)
                foreach (var v in value.EnumerateArray()) arg.Values.Add(v.GetString() ?? "");
        }
        return arg;
    }
}

/// <summary>arguments.game/jvm 既可能是字符串，也可能是 { rules, value } 对象，需要自定义转换器。</summary>
public sealed class ArgumentConverter : JsonConverter<Argument>
{
    public override Argument Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        return Argument.FromJson(doc.RootElement);
    }

    public override void Write(Utf8JsonWriter writer, Argument value, JsonSerializerOptions options)
    {
        if (value.Rules is null || value.Rules.Count == 0)
        {
            if (value.Values.Count == 1) writer.WriteStringValue(value.Values[0]);
            else
            {
                writer.WriteStartArray();
                foreach (var item in value.Values) writer.WriteStringValue(item);
                writer.WriteEndArray();
            }
            return;
        }

        writer.WriteStartObject();
        writer.WritePropertyName("rules");
        JsonSerializer.Serialize(writer, value.Rules, options);
        writer.WritePropertyName("value");
        if (value.Values.Count == 1) writer.WriteStringValue(value.Values[0]);
        else
        {
            writer.WriteStartArray();
            foreach (var item in value.Values) writer.WriteStringValue(item);
            writer.WriteEndArray();
        }
        writer.WriteEndObject();
    }
}

/// <summary>兼容 Fabric/Quilt 元数据里的 "+0000" 写法（标准 ISO 需要 "+00:00"）。</summary>
public sealed class LenientDateTimeOffsetConverter : JsonConverter<DateTimeOffset?>
{
    public override DateTimeOffset? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null) return null;
        var text = reader.GetString();
        if (string.IsNullOrWhiteSpace(text)) return null;
        if (System.Text.RegularExpressions.Regex.IsMatch(text, @"[+-]\d{4}$"))
            text = text[..^2] + ":" + text[^2..];
        return DateTimeOffset.TryParse(text, System.Globalization.CultureInfo.InvariantCulture,
            System.Globalization.DateTimeStyles.RoundtripKind, out var value) ? value : null;
    }

    public override void Write(Utf8JsonWriter writer, DateTimeOffset? value, JsonSerializerOptions options)
    {
        if (value is null) writer.WriteNullValue();
        else writer.WriteStringValue(value.Value);
    }
}

public sealed class JavaVersion
{
    [JsonPropertyName("component")] public string Component { get; set; } = "";
    [JsonPropertyName("majorVersion")] public int MajorVersion { get; set; }
}

public sealed class LoggingConfig
{
    [JsonPropertyName("client")] public LoggingFile Client { get; set; } = new();
}

public sealed class LoggingFile
{
    [JsonPropertyName("argument")] public string Argument { get; set; } = "";
    [JsonPropertyName("type")] public string Type { get; set; } = "";
    [JsonPropertyName("file")] public Artifact File { get; set; } = new();
}

public sealed class AssetIndex
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("objects")] public Dictionary<string, AssetObject> Objects { get; set; } = [];
}

public sealed class AssetObject
{
    [JsonPropertyName("hash")] public string Hash { get; set; } = "";
    [JsonPropertyName("size")] public long Size { get; set; }
}

public sealed class JavaRuntimeManifest
{
    [JsonPropertyName("files")] public Dictionary<string, JavaRuntimeFile> Files { get; set; } = [];
}

public sealed class JavaRuntimeFile
{
    [JsonPropertyName("type")] public string Type { get; set; } = "file";
    [JsonPropertyName("executable")] public bool Executable { get; set; }
    [JsonPropertyName("downloads")] public JavaRuntimeDownloads? Downloads { get; set; }
}

public sealed class JavaRuntimeDownloads
{
    [JsonPropertyName("raw")] public Artifact Raw { get; set; } = new();
}
