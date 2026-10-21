using System.Windows;
using DLP.App.Services;
using DLP.App.ViewModels;

namespace DLP.App;

public partial class App : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        var services = new AppServices();
        var viewModel = new MainViewModel(
            services,
            new HomeViewModel(services),
            new DownloadViewModel(services),
            new VersionsViewModel(services),
            new SettingsViewModel(services));

        var window = new MainWindow { DataContext = viewModel };
        MainWindow = window;
        window.Show();

        ThemeManager.Apply(services.Settings.Theme);
    }
}
