namespace DLP.Core.Minecraft;

/// <summary>按 Mojang 规则求值：无规则 = 允许；有规则时“最后一条匹配的规则生效”，无匹配 = 禁止。</summary>
public static class RuleEvaluator
{
    public static bool IsAllowed(IReadOnlyList<Rule>? rules, OsContext os, IReadOnlyDictionary<string, bool>? features = null)
    {
        if (rules is null || rules.Count == 0) return true;

        bool allowed = false;
        bool matched = false;
        foreach (var rule in rules)
        {
            if (!Matches(rule, os, features ?? EmptyFeatures)) continue;
            matched = true;
            allowed = string.Equals(rule.Action, "allow", StringComparison.OrdinalIgnoreCase);
        }
        return matched && allowed;
    }

    private static readonly IReadOnlyDictionary<string, bool> EmptyFeatures = new Dictionary<string, bool>();

    public static bool Matches(Rule rule, OsContext os, IReadOnlyDictionary<string, bool> features)
    {
        if (rule.Os is { } osRule)
        {
            if (osRule.Name is { } name && !string.Equals(name, os.Name, StringComparison.OrdinalIgnoreCase)) return false;
            if (osRule.Arch is { } arch && !string.Equals(arch, os.Arch, StringComparison.OrdinalIgnoreCase)) return false;
            if (osRule.Version is { } version && !System.Text.RegularExpressions.Regex.IsMatch(os.Version, version)) return false;
        }
        if (rule.Features is { } ruleFeatures)
        {
            foreach (var (key, expected) in ruleFeatures)
            {
                if (!features.TryGetValue(key, out var actual) || actual != expected) return false;
            }
        }
        return true;
    }
}

public sealed record OsContext(string Name, string Arch, string Version)
{
    public static OsContext Windows { get; } = new(
        "windows",
        Environment.Is64BitOperatingSystem ? "x86_64" : "x86",
        Environment.OSVersion.Version.ToString());
}
