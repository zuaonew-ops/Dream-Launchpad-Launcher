using System.Diagnostics;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Avalonia.Services;

namespace DLP.Avalonia.ViewModels;

public sealed class SettingsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private string _theme;
    private int _memoryMb;
    private string _username;
    private string _source;

    public SettingsViewModel(AppServices services)
    {
        _services = services;
        _theme = services.Settings.Theme == ThemeManager.Dark ? "暗色" : "亮色";
        _memoryMb = Math.Clamp(services.Settings.MemoryMb, 512, MaxMemoryMb);
        _username = services.Settings.Username;
        _source = services.Settings.DownloadSource == "official" ? "官方 CDN（推荐）" : "镜像加速";

        Themes = ["亮色", "暗色"];
        Sources = ["官方 CDN（推荐）", "镜像加速"];
        OpenDataFolderCommand = new RelayCommand(() => Shell.Open(_services.Paths.Root));
    }

    public int MaxMemoryMb => SystemMemory.MaxAllocatableMb;
    public string MemoryLimitText => SystemMemory.LimitText;

    public IReadOnlyList<string> Themes { get; }
    public IReadOnlyList<string> Sources { get; }
    public RelayCommand OpenDataFolderCommand { get; }
    public string DataRoot => _services.Paths.Root;
    public string Version => "bata0.1.0";

    public string Theme
    {
        get => _theme;
        set
        {
            if (!SetProperty(ref _theme, value)) return;
            _services.Settings.Theme = value == "暗色" ? ThemeManager.Dark : ThemeManager.Light;
            _services.Settings.Save();
            ThemeManager.Apply(_services.Settings.Theme);
        }
    }

    public int MemoryMb
    {
        get => _memoryMb;
        set
        {
            if (!SetProperty(ref _memoryMb, value)) return;
            _services.Settings.MemoryMb = value;
            _services.Settings.Save();
        }
    }

    public string Username
    {
        get => _username;
        set
        {
            if (!SetProperty(ref _username, value)) return;
            _services.Settings.Username = value;
            _services.Settings.Save();
        }
    }

    public string Source
    {
        get => _source;
        set
        {
            if (!SetProperty(ref _source, value)) return;
            _services.Settings.DownloadSource = value.StartsWith("官方") ? "official" : "mirror";
            _services.Settings.Save();
            DLP.Core.Download.Mirrors.Enabled = _services.Settings.DownloadSource != "official";
        }
    }
}
