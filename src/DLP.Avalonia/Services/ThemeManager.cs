using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;

namespace DLP.Avalonia.Services;

/// <summary>亮色/暗色主题：替换调色板里的画刷，界面用 DynamicResource 自然跟着变。</summary>
public static class ThemeManager
{
    public const string Light = "light";
    public const string Dark = "dark";

    public static string Current { get; private set; } = Light;

    private static readonly Dictionary<string, (string Light, string Dark)> Palette = new()
    {
        ["WindowBg"] = ("#F6F6F7", "#232427"),
        ["Surface"] = ("#FFFFFF", "#2A2C30"),
        ["SurfaceAlt"] = ("#EFEFF1", "#313337"),
        ["SurfaceMuted"] = ("#F1F2F4", "#34363B"),
        ["HoverBg"] = ("#EDEEF0", "#3A3C41"),
        ["Line"] = ("#E2E3E6", "#3E4146"),
        ["TextBrush"] = ("#191A1C", "#ECEDEF"),
        ["TextSecondary"] = ("#5E6369", "#A9AEB5"),
        ["TextTertiary"] = ("#8B9097", "#7C828A"),
        ["Accent"] = ("#2F9E63", "#3FB476"),
        ["AccentHover"] = ("#278754", "#48C284"),
        ["AccentSoft"] = ("#E8F4ED", "#2C3B33"),
        ["Danger"] = ("#C0392B", "#E06A5C")
    };

    public static void Apply(string? theme)
    {
        Current = string.Equals(theme, Dark, StringComparison.OrdinalIgnoreCase) ? Dark : Light;
        var dark = Current == Dark;

        if (Application.Current is not { } app) return;
        app.RequestedThemeVariant = dark ? ThemeVariant.Dark : ThemeVariant.Light;

        foreach (var (key, pair) in Palette)
        {
            var brush = new SolidColorBrush(Color.Parse(dark ? pair.Dark : pair.Light));
            app.Resources[key] = brush;
        }
    }
}
