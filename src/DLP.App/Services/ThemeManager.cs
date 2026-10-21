using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;

namespace DLP.App.Services;

/// <summary>亮色 / 暗色主题：只换调色板里那几个画刷，界面用 DynamicResource 自动跟着变。</summary>
public static class ThemeManager
{
    public const string Light = "light";
    public const string Dark = "dark";

    private static readonly (string Key, string Light, string Dark)[] Palette =
    [
        ("WindowBg", "#F6F6F7", "#1B1C1E"),
        ("Surface", "#FFFFFF", "#232427"),
        ("SurfaceAlt", "#EFEFF1", "#2A2C30"),
        ("SurfaceMuted", "#F1F2F4", "#2E3034"),
        ("HoverBg", "#EDEEF0", "#32353A"),
        ("Line", "#E2E3E6", "#3A3D42"),
        ("TextBrush", "#191A1C", "#ECEDEF"),
        ("TextSecondary", "#5E6369", "#A9AEB6"),
        ("TextTertiary", "#8B9097", "#7F858D"),
        ("Accent", "#2F9E63", "#3AB373"),
        ("AccentHover", "#278754", "#2F9E63"),
        ("AccentSoft", "#E8F4ED", "#1E3A2B"),
        ("Danger", "#C0392B", "#E06C5F"),
    ];

    public static string Current { get; private set; } = Light;

    public static void Apply(string? theme)
    {
        var dark = string.Equals(theme, Dark, StringComparison.OrdinalIgnoreCase);
        Current = dark ? Dark : Light;
        var resources = Application.Current?.Resources;
        if (resources is null) return;

        foreach (var (key, light, darkValue) in Palette)
        {
            var color = (Color)ColorConverter.ConvertFromString(dark ? darkValue : light);
            var brush = new SolidColorBrush(color);
            brush.Freeze();
            resources[key] = brush;
        }

        foreach (Window window in Application.Current!.Windows)
            ApplyTitleBar(window, dark);
    }

    /// <summary>标题栏跟着变（Win10 1809+ 的 immersive dark mode 属性）。</summary>
    public static void ApplyTitleBar(Window window, bool dark)
    {
        try
        {
            var handle = new WindowInteropHelper(window).Handle;
            if (handle == IntPtr.Zero) return;
            var value = dark ? 1 : 0;
            DwmSetWindowAttribute(handle, 20, ref value, sizeof(int));
            DwmSetWindowAttribute(handle, 19, ref value, sizeof(int));
        }
        catch
        {
            // 老系统没有这个属性，忽略即可。
        }
    }

    [System.Runtime.InteropServices.DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
}
