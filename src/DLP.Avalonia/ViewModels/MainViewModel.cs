using System.Collections.ObjectModel;
using Avalonia.Threading;
using DLP.App.Mvvm;
using DLP.Avalonia.Services;

namespace DLP.Avalonia.ViewModels;

/// <summary>一个左侧导航项。</summary>
public sealed class NavItem(string title, object page) : ObservableObject
{
    public string Title { get; } = title;
    public object Page { get; } = page;
}

public sealed class MainViewModel : ObservableObject
{
    private NavItem? _selectedNav;

    public MainViewModel(AppServices services, HomeViewModel home, SettingsViewModel settings)
    {
        Services = services;
        Download = new DownloadViewModel(services);
        Versions = new VersionsViewModel(services);
        NavItems =
        [
            new NavItem("首页", home),
            new NavItem("下载", Download),
            new NavItem("游戏版本", Versions),
            new NavItem("设置", settings)
        ];
        SelectedNav = NavItems[0];

        // 用启动器打开整合包：命令行 --import "路径" 直接进「下载 → 本地文件」。
        TryHandleImportArgument();

        // 左下角的当前账号跟随账号切换刷新。
        Services.Accounts.Changed += () =>
        {
            if (!Dispatcher.UIThread.CheckAccess())
            {
                Dispatcher.UIThread.Post(() => OnPropertyChanged(nameof(AccountName)));
                return;
            }

            OnPropertyChanged(nameof(AccountName));
        };
    }

    public AppServices Services { get; }

    /// <summary>「下载」页的视图模型（首次显示时再拉一次版本清单）。</summary>
    public DownloadViewModel Download { get; }

    /// <summary>「游戏版本」页的视图模型。</summary>
    public VersionsViewModel Versions { get; }

    public ObservableCollection<NavItem> NavItems { get; }

    public NavItem? SelectedNav
    {
        get => _selectedNav;
        set
        {
            if (!SetProperty(ref _selectedNav, value)) return;
            OnPropertyChanged(nameof(CurrentPage));
            OnPropertyChanged(nameof(CurrentTitle));
            if (ReferenceEquals(_selectedNav?.Page, Download)) Download.EnsureLoaded();
        }
    }

    public string AccountName => Services.Accounts.CurrentName;
    public object? CurrentPage => SelectedNav?.Page;
    public string CurrentTitle => SelectedNav?.Title ?? "";

    /// <summary>处理 --import 路径：切到「下载」页并开始安装本地整合包。</summary>
    private void TryHandleImportArgument()
    {
        var args = Environment.GetCommandLineArgs();
        var index = Array.FindIndex(args, a => string.Equals(a, "--import", StringComparison.OrdinalIgnoreCase));
        if (index < 0 || index + 1 >= args.Length) return;

        var packPath = args[index + 1];
        if (!System.IO.File.Exists(packPath)) return;

        SelectedNav = NavItems.FirstOrDefault(n => ReferenceEquals(n.Page, Download));
        Download.StartLocalImport(packPath);
    }
}
