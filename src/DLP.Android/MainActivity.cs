using Android.App;
using Android.Content.PM;
using Avalonia;
using Avalonia.Android;

namespace DLP.Android;

[Activity(
    Label = "Dream Launchpad",
    Theme = "@style/MyTheme.NoActionBar",
    MainLauncher = true,
    ConfigurationChanges = ConfigChanges.Orientation | ConfigChanges.ScreenSize | ConfigChanges.UiMode)]
public class MainActivity : AvaloniaMainActivity<global::DLP.Avalonia.App>
{
    protected override AppBuilder CustomizeAppBuilder(AppBuilder builder)
        => base.CustomizeAppBuilder(builder).WithInterFont();
}
