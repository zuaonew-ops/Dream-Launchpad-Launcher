using System.Collections.ObjectModel;
using System.IO;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Core.Download;
using DLP.Core.Install;

namespace DLP.App.ViewModels;

public sealed record DownloadCategory(string Title, string Kind)
{
    public bool IsVersion => Kind == "version";
}

public sealed class DownloadViewModel : ObservableObject
{
    private readonly AppServices _services;
    private readonly LoaderService _loaders;
    private DownloadCategory _selectedCategory;
    private string? _selectedGameVersion;
    private string _selectedLoader = "原版";
    private LoaderVersion? _selectedLoaderVersion;
    private ContentProject? _selectedProject;
    private string _search = "";
    private string _status = "选择分类开始浏览";
    private double _progress;
    private bool _isBusy;
    private CancellationTokenSource? _loaderCts;
    private CancellationTokenSource? _searchCts;

    public DownloadViewModel(AppServices services)
    {
        _services = services;
        _loaders = new LoaderService(services.Paths, services.Http);

        Categories = [new("游戏版本", "version"), new("模组", "mod"), new("整合包", "modpack"),
            new("光影", "shader"), new("资源包", "resourcepack"), new("数据包", "datapack")];
        _selectedCategory = Categories[0];

        ManifestVersions = [];
        FilteredVersions = [];
        LoaderVersions = [];
        Results = [];

        RefreshCommand = new AsyncRelayCommand(LoadManifestAsync);
        LoadLoadersCommand = new AsyncRelayCommand(LoadLoadersAsync);
        SearchCommand = new AsyncRelayCommand(SearchAsync);
        InstallVersionCommand = new AsyncRelayCommand(InstallVersionAsync, () => !IsBusy && SelectedGameVersion is not null);
        InstallContentCommand = new AsyncRelayCommand(InstallContentAsync, () => !IsBusy && SelectedProject is not null);

        _ = LoadManifestAsync();
    }

    public ObservableCollection<DownloadCategory> Categories { get; }
    public ObservableCollection<string> ManifestVersions { get; }
    public ObservableCollection<string> FilteredVersions { get; }
    public ObservableCollection<LoaderVersion> LoaderVersions { get; }
    public ObservableCollection<ContentProject> Results { get; }
    public AsyncRelayCommand RefreshCommand { get; }
    public AsyncRelayCommand LoadLoadersCommand { get; }
    public AsyncRelayCommand SearchCommand { get; }
    public AsyncRelayCommand InstallVersionCommand { get; }
    public AsyncRelayCommand InstallContentCommand { get; }

    public string[] LoaderNames { get; } = ["原版", "Fabric", "Quilt", "Forge", "NeoForge"];

    public DownloadCategory SelectedCategory
    {
        get => _selectedCategory;
        set
        {
            if (!SetProperty(ref _selectedCategory, value)) return;
            OnPropertyChanged(nameof(IsVersionTab));
            OnPropertyChanged(nameof(IsContentTab));
            OnPropertyChanged(nameof(ShowLoader));
            Results.Clear();
            SelectedProject = null;
            Status = value.IsVersion ? "选择版本后安装" : "搜索内容后安装";
            if (value.IsVersion) _ = LoadManifestAsync();
        }
    }

    public bool IsVersionTab => SelectedCategory.IsVersion;
    public bool IsContentTab => !SelectedCategory.IsVersion;
    public bool ShowLoader => IsVersionTab || SelectedCategory.Kind == "mod";

    public string? SelectedGameVersion
    {
        get => _selectedGameVersion;
        set
        {
            if (!SetProperty(ref _selectedGameVersion, value)) return;
            InstallVersionCommand.RaiseCanExecuteChanged();
            _ = LoadLoadersAsync();
        }
    }

    public string SelectedLoader
    {
        get => _selectedLoader;
        set
        {
            if (!SetProperty(ref _selectedLoader, value)) return;
            _ = LoadLoadersAsync();
        }
    }

    public LoaderVersion? SelectedLoaderVersion
    {
        get => _selectedLoaderVersion;
        set => SetProperty(ref _selectedLoaderVersion, value);
    }

    public ContentProject? SelectedProject
    {
        get => _selectedProject;
        set { if (SetProperty(ref _selectedProject, value)) InstallContentCommand.RaiseCanExecuteChanged(); }
    }

    public string Search
    {
        get => _search;
        set
        {
            if (!SetProperty(ref _search, value)) return;
            if (IsVersionTab) ApplyVersionFilter();
        }
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

    public bool IsBusy
    {
        get => _isBusy;
        set
        {
            if (!SetProperty(ref _isBusy, value)) return;
            InstallVersionCommand.RaiseCanExecuteChanged();
            InstallContentCommand.RaiseCanExecuteChanged();
        }
    }

    private async Task LoadManifestAsync()
    {
        try
        {
            Status = "加载版本清单…";
            var manifest = await _services.Manifests.GetAsync();
            var previous = SelectedGameVersion;
            ManifestVersions.Clear();
            FilteredVersions.Clear();
            foreach (var entry in manifest.Versions)
            {
                ManifestVersions.Add(entry.Id);
                FilteredVersions.Add(entry.Id);
            }
            var fallback = previous is not null && ManifestVersions.Contains(previous)
                ? previous
                : manifest.Versions.FirstOrDefault(v => v.Type == "release")?.Id ?? ManifestVersions.FirstOrDefault();
            await Application.Current.Dispatcher.InvokeAsync(() => SelectedGameVersion = fallback);
            Status = $"共 {ManifestVersions.Count} 个版本";
            if (IsVersionTab && Search.Trim().Length > 0) ApplyVersionFilter();
        }
        catch (Exception ex)
        {
            Status = "清单加载失败：" + ex.Message;
        }
    }

    private ModLoaderType? CurrentLoaderType => SelectedLoader switch
    {
        "Fabric" => ModLoaderType.Fabric,
        "Quilt" => ModLoaderType.Quilt,
        "Forge" => ModLoaderType.Forge,
        "NeoForge" => ModLoaderType.NeoForge,
        _ => null
    };

    private async Task LoadLoadersAsync()
    {
        _loaderCts?.Cancel();
        _loaderCts?.Dispose();
        _loaderCts = new CancellationTokenSource(TimeSpan.FromSeconds(30));
        var token = _loaderCts.Token;

        LoaderVersions.Clear();
        SelectedLoaderVersion = null;
        if (CurrentLoaderType is not { } type || SelectedGameVersion is not { } mc) return;

        try
        {
            Status = $"加载 {SelectedLoader} 版本列表…";
            // 快速连点下拉框时只保留最后一次请求。
            await Task.Delay(250, token);
            var list = await _loaders.ListAsync(type, mc, token);
            if (token.IsCancellationRequested) return;
            foreach (var item in list) LoaderVersions.Add(item);
            SelectedLoaderVersion = LoaderVersions.FirstOrDefault();
            Status = $"{SelectedLoader} 共 {LoaderVersions.Count} 个构建";
        }
        catch (OperationCanceledException)
        {
            if (!token.IsCancellationRequested) Status = $"{SelectedLoader} 列表加载超时";
        }
        catch (Exception ex)
        {
            if (!token.IsCancellationRequested) Status = $"{SelectedLoader} 列表加载失败：" + ex.Message;
        }
    }

    private void ApplyVersionFilter()
    {
        var keyword = Search.Trim();
        var previous = SelectedGameVersion;
        FilteredVersions.Clear();
        foreach (var id in ManifestVersions)
            if (keyword.Length == 0 || id.Contains(keyword, StringComparison.OrdinalIgnoreCase))
                FilteredVersions.Add(id);

        Status = keyword.Length == 0
            ? $"共 {ManifestVersions.Count} 个版本，输入版本号过滤"
            : $"匹配 {FilteredVersions.Count} / {ManifestVersions.Count} 个版本";

        SelectedGameVersion = previous is not null && FilteredVersions.Contains(previous)
            ? previous
            : FilteredVersions.FirstOrDefault();
    }

    private async Task SearchAsync()
    {
        if (IsVersionTab)
        {
            ApplyVersionFilter();
            return;
        }
        _searchCts?.Cancel();
        _searchCts?.Dispose();
        _searchCts = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        var token = _searchCts.Token;

        IsBusy = true;
        try
        {
            Status = "搜索中…";
            var kind = SelectedCategory.Kind switch
            {
                "mod" => ContentKind.Mod,
                "modpack" => ContentKind.Modpack,
                "shader" => ContentKind.Shader,
                "resourcepack" => ContentKind.ResourcePack,
                _ => ContentKind.DataPack
            };
            var loader = kind == ContentKind.Mod && SelectedLoader != "原版" ? SelectedLoader.ToLowerInvariant() : null;
            var list = await _services.Modrinth.SearchAsync(kind, Search, SelectedGameVersion, loader, 30, token);
            Results.Clear();
            foreach (var item in list) Results.Add(item);
            Status = list.Count == 0 ? "没有找到匹配内容" : $"找到 {list.Count} 个结果";
        }
        catch (OperationCanceledException)
        {
            Status = "搜索超时";
        }
        catch (Exception ex)
        {
            if (!token.IsCancellationRequested) Status = "搜索失败：" + ex.Message;
        }
        finally { IsBusy = false; }
    }

    private async Task InstallVersionAsync()
    {
        if (SelectedGameVersion is not { } mc) return;
        IsBusy = true;
        Progress = 0;
        try
        {
            Status = $"安装原版 {mc}…";
            await _services.Installer.InstallAsync(mc, new Progress<InstallProgress>(p =>
            {
                Progress = p.Total > 0 ? (double)p.Completed / p.Total : 0;
                Status = p.Stage;
            }));

            _services.NotifyInstalledVersionsChanged();

            if (CurrentLoaderType is { } type)
            {
                var loaderVersion = SelectedLoaderVersion?.Id;
                if (string.IsNullOrWhiteSpace(loaderVersion))
                {
                    Status = "未选择加载器版本";
                    return;
                }
                Status = $"安装 {SelectedLoader} {loaderVersion}…";
                var id = await _loaders.InstallAsync(type, mc, loaderVersion,
                    new Progress<string>(s => Status = s), CancellationToken.None);
                var installed = _services.GetInstalledVersions();
                var actual = installed.FirstOrDefault(v => v.Equals(id, StringComparison.OrdinalIgnoreCase))
                    ?? installed.FirstOrDefault(v => v.Contains(mc) && v.Contains(SelectedLoader.ToLowerInvariant()));
                Status = actual is not null ? $"已安装 {actual}" : $"已安装 {id}";
            }
            else
            {
                Status = $"已安装 {mc}";
            }

            _services.NotifyInstalledVersionsChanged();
        }
        catch (Exception ex)
        {
            Status = "安装失败：" + ex.Message;
        }
        finally
        {
            IsBusy = false;
        }
    }

    private async Task InstallContentAsync()
    {
        if (SelectedProject is not { } project || SelectedGameVersion is not { } mc) return;
        IsBusy = true;
        Progress = 0;
        try
        {
            var gameDir = _services.Paths.GameDir(mc);
            if (!Directory.Exists(_services.Paths.VersionDir(mc)))
            {
                Status = $"请先安装 {mc} 再下载内容";
                return;
            }
            var kind = SelectedCategory.Kind switch
            {
                "mod" => ContentKind.Mod,
                "modpack" => ContentKind.Modpack,
                "shader" => ContentKind.Shader,
                "resourcepack" => ContentKind.ResourcePack,
                _ => ContentKind.DataPack
            };
            var loader = kind == ContentKind.Mod && SelectedLoader != "原版" ? SelectedLoader.ToLowerInvariant() : null;
            Status = $"下载 {project.Title}…";
            var path = await _services.Modrinth.InstallAsync(kind, project.Id, mc, loader, gameDir,
                new Progress<double>(v => Progress = v), CancellationToken.None);
            Status = "已下载：" + path;
        }
        catch (Exception ex)
        {
            Status = "下载失败：" + ex.Message;
        }
        finally { IsBusy = false; }
    }
}
