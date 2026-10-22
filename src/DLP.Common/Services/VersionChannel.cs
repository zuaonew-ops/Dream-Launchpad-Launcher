namespace DLP.App.Services;

/// <summary>版本渠道分类：正式版 / 快照版 / 愚人节版 / 旧版 / 其它。</summary>
public static class VersionChannel
{
    public const string Release = "正式版";
    public const string Snapshot = "快照版";
    public const string AprilFools = "愚人节版";
    public const string Legacy = "旧版";

    /// <summary>愚人节版本（Mojang 清单里类型都是 snapshot，只能按 id 认）。</summary>
    private static readonly HashSet<string> AprilFoolsIds = new(StringComparer.OrdinalIgnoreCase)
    {
        "15w14a", "1.RV-Pre1", "3D Shareware v1.34", "20w14infinite",
        "22w13oneBlockAtATime", "23w13a_or_b", "24w14potato", "25w14craftmine",
        "2.0", "1.14_combat-212796", "1.14_combat-0", "1.14_combat-3",
    };

    public static bool IsAprilFools(string id) => AprilFoolsIds.Contains(id);

    /// <summary>把清单里的 type 字段 + 版本 id 归到渠道。</summary>
    public static string Classify(string id, string? type)
    {
        if (IsAprilFools(id)) return AprilFools;
        if (string.Equals(type, "release", StringComparison.OrdinalIgnoreCase)) return Release;
        if (string.Equals(type, "snapshot", StringComparison.OrdinalIgnoreCase)) return Snapshot;
        return Legacy;
    }
}
