using System.Collections.ObjectModel;
using DLP.App.Mvvm;
using DLP.App.Services;

namespace DLP.App.ViewModels;

public sealed record NavItem(string Title, string Glyph, ObservableObject Page);

public sealed class MainViewModel : ObservableObject
{
    private NavItem? _selectedNav;

    public ObservableCollection<NavItem> NavItems { get; }
    public ObservableCollection<AccountEntry> Accounts => Services.Accounts.Accounts;

    /// <summary>右下角下载任务弹窗的数据源。</summary>
    public DownloadMonitor Downloads => Services.Downloads;
    public string AccountName => Services.Accounts.CurrentName;
    public string AccountKind => Services.Accounts.CurrentKind;
    public string AccountInitial => Services.Accounts.CurrentInitial;

    public MainViewModel(AppServices services, HomeViewModel home, DownloadViewModel download,
        VersionsViewModel versions, SettingsViewModel settings)
    {
        Services = services;
        NavItems =
        [
            new NavItem("首页", "\uE80F", home),
            new NavItem("下载", "\uE896", download),
            new NavItem("游戏版本", "\uE8B7", versions),
            new NavItem("设置", "\uE713", settings)
        ];
        Services.Log.Info("导航初始化，首页");
        SelectedNav = NavItems[0];
        Services.Accounts.Changed += () =>
        {
            OnPropertyChanged(nameof(AccountName));
            OnPropertyChanged(nameof(AccountKind));
            OnPropertyChanged(nameof(AccountInitial));
            OnPropertyChanged(nameof(CurrentAccount));
        };
    }

    public AppServices Services { get; }

    public AccountEntry? CurrentAccount
    {
        get => Services.Accounts.Current;
        set { if (value is not null) Services.Accounts.Switch(value); }
    }

    public NavItem? SelectedNav
    {
        get => _selectedNav;
        set
        {
            if (!SetProperty(ref _selectedNav, value)) return;
            Services.Log.Info($"导航切换到 {value?.Title ?? "null"}");
            OnPropertyChanged(nameof(CurrentPage));
        }
    }

    public ObservableObject? CurrentPage => SelectedNav?.Page;
}
