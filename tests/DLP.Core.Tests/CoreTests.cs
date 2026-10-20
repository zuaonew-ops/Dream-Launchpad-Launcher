using DLP.Core.Auth;
using DLP.Core.Minecraft;

namespace DLP.Core.Tests;

public class RuleEvaluatorTests
{
    private static readonly OsContext Windows = new("windows", "x86_64", "10.0.22631");
    private static readonly OsContext Mac = new("osx", "x86_64", "14.0");

    [Fact]
    public void NoRules_IsAllowed()
        => Assert.True(RuleEvaluator.IsAllowed(null, Windows));

    [Fact]
    public void AllowWindows_Matches()
    {
        var rules = new List<Rule> { new() { Action = "allow", Os = new OsRule { Name = "windows" } } };
        Assert.True(RuleEvaluator.IsAllowed(rules, Windows));
        Assert.False(RuleEvaluator.IsAllowed(rules, Mac));
    }

    [Fact]
    public void AllowOsxOnly_IsDisallowedOnWindows()
    {
        var rules = new List<Rule> { new() { Action = "allow", Os = new OsRule { Name = "osx" } } };
        Assert.False(RuleEvaluator.IsAllowed(rules, Windows));
    }

    [Fact]
    public void DisallowThenAllow_WindowsWins()
    {
        var rules = new List<Rule>
        {
            new() { Action = "disallow", Os = new OsRule { Name = "windows" } },
            new() { Action = "allow", Os = new OsRule { Name = "windows" } }
        };
        Assert.True(RuleEvaluator.IsAllowed(rules, Windows));
    }

    [Fact]
    public void FeatureRules_MatchContext()
    {
        var rules = new List<Rule>
        {
            new() { Action = "allow", Features = new Dictionary<string, bool> { ["is_demo_user"] = true } }
        };
        Assert.False(RuleEvaluator.IsAllowed(rules, Windows, new Dictionary<string, bool>()));
        Assert.True(RuleEvaluator.IsAllowed(rules, Windows, new Dictionary<string, bool> { ["is_demo_user"] = true }));
    }
}

public class MavenCoordinateTests
{
    [Fact]
    public void ParsesSimpleCoordinate()
    {
        var coordinate = MavenCoordinate.Parse("org.lwjgl:lwjgl:3.3.3");
        Assert.Equal("org.lwjgl:lwjgl:3.3.3", $"{coordinate.Group}:{coordinate.Artifact}:{coordinate.Version}");
        Assert.Equal("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar", coordinate.ToPath());
    }

    [Fact]
    public void ParsesClassifierAndExtension()
    {
        var coordinate = MavenCoordinate.Parse("org.lwjgl:lwjgl:3.3.3:natives-windows@jar");
        Assert.Equal("natives-windows", coordinate.Classifier);
        Assert.Equal("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar", coordinate.ToPath());
    }

    [Fact]
    public void RejectsInvalidCoordinate()
        => Assert.Throws<FormatException>(() => MavenCoordinate.Parse("bad"));
}

public class OfflineAccountTests
{
    [Fact]
    public void GeneratesStableUuid()
    {
        var first = LaunchAccount.Offline("Steve");
        var second = LaunchAccount.Offline("Steve");
        Assert.Equal(first.Uuid, second.Uuid);
        Assert.Equal(32, first.Uuid.Length);
        Assert.Equal("0", first.AccessToken);
        Assert.Equal("legacy", first.UserType);
    }

    [Fact]
    public void DifferentNames_DifferentUuid()
        => Assert.NotEqual(LaunchAccount.Offline("Steve").Uuid, LaunchAccount.Offline("Alex").Uuid);
}
