using System.IO;
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

        // 用启动器打开整合包：`DLP.App.exe --import "D:\path\pack.zip"`，直接进「下载 → 本地文件」显示进度。
        var importIndex = Array.FindIndex(e.Args, a => string.Equals(a, "--import", StringComparison.OrdinalIgnoreCase));
        if (importIndex >= 0 && importIndex + 1 < e.Args.Length)
        {
            var packPath = e.Args[importIndex + 1];
            if (File.Exists(packPath))
            {
                var download = viewModel.NavItems.Select(n => n.Page).OfType<DownloadViewModel>().FirstOrDefault();
                if (download is not null)
                {
                    viewModel.SelectedNav = viewModel.NavItems.FirstOrDefault(n => ReferenceEquals(n.Page, download));
                    download.StartLocalImport(packPath);
                }
            }
        }
    }
}
