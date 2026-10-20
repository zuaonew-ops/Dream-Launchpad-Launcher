using System.IO;
using System.Collections.ObjectModel;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Core.Install;
using DLP.Core.Minecraft;

namespace DLP.App.ViewModels;

public sealed class VersionsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private readonly List<VersionEntry> _all = [];
    private string _search = "";
    private string _typeFilter = "正式版";
    private VersionEntry? _selected;
    private bool _isLoading;
    private bool _isInstalling;
    private string _status = "正在加载版本清单…";
    private double _progress;

    public VersionsViewModel(AppServices services)
    {
        _services = services;
        Versions = [];
        LoadCommand = new AsyncRelayCommand(LoadAsync);
        InstallCommand = new AsyncRelayCommand(InstallAsync, () => SelectedVersion is not null && !IsInstalling);
        _ = LoadAsync();
    }

    public ObservableCollection<VersionEntry> Versions { get; }
    public AsyncRelayCommand LoadCommand { get; }
    public AsyncRelayCommand InstallCommand { get; }
    public string[] TypeFilters { get; } = ["正式版", "快照", "全部"];

    public string Search
    {
        get => _search;
        set { if (SetProperty(ref _search, value)) ApplyFilter(); }
    }

    public string TypeFilter
    {
        get => _typeFilter;
        set { if (SetProperty(ref _typeFilter, value)) ApplyFilter(); }
    }

    public VersionEntry? SelectedVersion
    {
        get => _selected;
        set
        {
            if (!SetProperty(ref _selected, value)) return;
            OnPropertyChanged(nameof(HasSelection));
            OnPropertyChanged(nameof(SelectedTitle));
            OnPropertyChanged(nameof(SelectedMeta));
            OnPropertyChanged(nameof(IsInstalled));
            InstallCommand.RaiseCanExecuteChanged();
        }
    }

    public bool HasSelection => SelectedVersion is not null;
    public string SelectedTitle => SelectedVersion?.Id ?? "选择一个版本";
    public string SelectedMeta => SelectedVersion is null ? "" : $"{SelectedVersion.Type} · {SelectedVersion.ReleaseTime:yyyy-MM-dd}";
    public bool IsInstalled => SelectedVersion is not null && File.Exists(_services.Paths.VersionJar(SelectedVersion.Id));

    public bool IsLoading
    {
        get => _isLoading;
        set => SetProperty(ref _isLoading, value);
    }

    public bool IsInstalling
    {
        get => _isInstalling;
        set { if (SetProperty(ref _isInstalling, value)) InstallCommand.RaiseCanExecuteChanged(); }
    }

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public double Progress
    {
        get => _progress;
        set => SetProperty(ref _progress, value);
    }

    private async Task LoadAsync()
    {
        IsLoading = true;
        Status = "正在加载版本清单…";
        try
        {
            var manifest = await _services.Manifests.GetAsync();
            _all.Clear();
            _all.AddRange(manifest.Versions);
            ApplyFilter();
            Status = $"共 {_all.Count} 个版本，最新正式版 {manifest.Latest.Release}";
        }
        catch (Exception ex)
        {
            Status = "版本清单加载失败：" + ex.Message;
        }
        finally
        {
            IsLoading = false;
        }
    }

    private void ApplyFilter()
    {
        IEnumerable<VersionEntry> query = _all;
        query = TypeFilter switch
        {
            "正式版" => query.Where(v => v.Type == "release"),
            "快照" => query.Where(v => v.Type == "snapshot"),
            _ => query
        };
        if (!string.IsNullOrWhiteSpace(Search))
        {
            var text = Search.Trim();
            query = query.Where(v => v.Id.Contains(text, StringComparison.OrdinalIgnoreCase));
        }

        Versions.Clear();
        foreach (var version in query) Versions.Add(version);
        if (SelectedVersion is null || !Versions.Contains(SelectedVersion))
            SelectedVersion = Versions.FirstOrDefault();
    }

    private async Task InstallAsync()
    {
        if (SelectedVersion is not { } entry) return;
        IsInstalling = true;
        Progress = 0;
        Status = $"正在安装 {entry.Id}…";

        try
        {
            var progress = new Progress<InstallProgress>(p =>
            {
                Progress = p.Total > 0 ? Math.Min(100, p.Completed * 100.0 / p.Total) : Progress;
                Status = $"{p.Stage}：{p.Completed}/{p.Total}";
            });
            await Task.Run(() => _services.Installer.InstallAsync(entry.Id, progress, CancellationToken.None), CancellationToken.None);
            var version = await _services.Versions.ResolveAsync(entry.Id);
            var runtime = await _services.Java.EnsureAsync(version);
            Status = $"安装完成：{entry.Id}（Java {runtime.MajorVersion}）";
            Progress = 100;
            _services.NotifyInstalledVersionsChanged();
            OnPropertyChanged(nameof(IsInstalled));
        }
        catch (Exception ex)
        {
            Status = "安装失败：" + ex.Message;
        }
        finally
        {
            IsInstalling = false;
        }
    }
}