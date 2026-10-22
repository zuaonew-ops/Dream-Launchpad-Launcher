using Avalonia;
using DLP.Avalonia;

namespace DLP.Desktop;

internal static class Program
{
    [STAThread]
    public static void Main(string[] args) => BuildAvaloniaApp()
        .StartWithClassicDesktopLifetime(args);

    public static AppBuilder BuildAvaloniaApp()
        => AppBuilder.Configure<global::DLP.Avalonia.App>()
            .UsePlatformDetect()
            .WithInterFont()
            .LogToTrace();
}
