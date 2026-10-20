using System.Collections.ObjectModel;
using DLP.App.Mvvm;
using DLP.App.Services;

namespace DLP.App.ViewModels;

public sealed record NavItem(string Title, string Glyph, ObservableObject Page);

public sealed class MainViewModel : ObservableObject
{
    private NavItem? _selectedNav;

    public ObservableCollection<NavItem> NavItems { get; }
    public string AccountName => Services.Settings.Username;

    public MainViewModel(AppServices services, HomeViewModel home, VersionsViewModel versions,
        InstancesViewModel instances, SettingsViewModel settings, LogViewModel logs)
    {
        Services = services;
        NavItems =
        [
            new NavItem("首页", "\uE80F", home),
            new NavItem("版本", "\uE8B7", versions),
            new NavItem("实例", "\uE8F1", instances),
            new NavItem("设置", "\uE713", settings),
            new NavItem("日志", "\uE9D9", logs)
        ];
        Services.Log.Info("导航初始化，首页");
        SelectedNav = NavItems[0];
    }

    public AppServices Services { get; }

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