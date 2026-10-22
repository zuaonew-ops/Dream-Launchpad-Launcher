using System.Collections.ObjectModel;
using System.IO;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Avalonia.Services;
using DLP.Core.Download;
using DLP.Core.Install;

namespace DLP.Avalonia.ViewModels;

/// <summary>「下载」页的一级分类（游戏版本 / 模组 / 整合包 / 光影 / 资源包 / 数据包）。</summary>
public sealed record DownloadCategory(string Title, string Kind)
{
    public bool IsVersion => Kind == "version";
}

/// <summary>
/// 下载页（Avalonia 版，对照 WPF 版 DLP.App.ViewModels.DownloadViewModel 移植）：
/// 游戏版本安装（版本号 + 加载器合并表单）、Modrinth 内容浏览与下载、本地整合包导入进度。
/// </summary>
public sealed class DownloadViewModel : ObservableObject
{
    private readonly AppServices _services;
    private readonly LoaderService _loaders;
    private readonly Dictionary<string, string> _versionChannels = new(StringComparer.OrdinalIgnoreCase);

    private DownloadCategory _selectedCategory;
    private string _selectedSubTab = "";
    private string? _selectedGameVersion;
    private string _selectedLoader = "原版";
    private LoaderVersion? _selectedLoaderVersion;
    private ContentItemViewModel? _selectedProject;
    private Bitmap? _selectedBanner;
    private string _search = "";
    private string _status = "选择分类开始浏览";
    private double _progress;
    private bool _isBusy;
    private bool _manifestLoading;
    private CancellationTokenSource? _loaderCts;
    private CancellationTokenSource? _searchCts;
    private DownloadTaskViewModel? _localImport;

    public DownloadViewModel(AppServices services)
    {
        _services = services;
        _loaders = services.Loaders;

        Categories =
        [
            new("游戏版本", "version"),
            new("模组", "mod"),
            new("整合包", "modpack"),
            new("光影", "shader"),
            new("资源包", "resourcepack"),
            new("数据包", "datapack")
        ];
        _selectedCategory = Categories[0];

        ManifestVersions = [];
        FilteredVersions = [];
        if (AvailableLoaders.Count == 0) AvailableLoaders.Add("原版");
        LoaderVersions = [];
        Results = [];

        RefreshCommand = new AsyncRelayCommand(LoadManifestAsync);
        LoadLoadersCommand = new AsyncRelayCommand(LoadLoadersAsync);
        SearchCommand = new AsyncRelayCommand(SearchAsync);
        InstallVersionCommand = new AsyncRelayCommand(InstallVersionAsync, () => !IsBusy && SelectedGameVersion is not null);
        InstallContentCommand = new AsyncRelayCommand(InstallContentAsync, () => !IsBusy && SelectedProject is not null);

        RebuildSubTabs();
        Status = "选好游戏版本和加载器，一次装好";
        _ = LoadManifestAsync();
    }

    public ObservableCollection<DownloadCategory> Categories { get; }
    public ObservableCollection<string> ManifestVersions { get; }
    public ObservableCollection<string> FilteredVersions { get; }
    public ObservableCollection<string> AvailableLoaders { get; } = [];
    public ObservableCollection<LoaderVersion> LoaderVersions { get; }
    public ObservableCollection<ContentItemViewModel> Results { get; }
    public ObservableCollection<string> SubTabs { get; } = [];
    public ObservableCollection<Bitmap> Gallery { get; } = [];

    public AsyncRelayCommand RefreshCommand { get; }
    public AsyncRelayCommand LoadLoadersCommand { get; }
    public AsyncRelayCommand SearchCommand { get; }
    public AsyncRelayCommand InstallVersionCommand { get; }
    public AsyncRelayCommand InstallContentCommand { get; }

    /// <summary>全局任务队列：下载详情面板与右下角弹窗共用。</summary>
    public DownloadMonitor Downloads => _services.Downloads;

    /// <summary>最近一次导入的整合包任务，进度显示在「整合包 → 本地文件」面板里。</summary>
    public DownloadTaskViewModel? LocalImport
    {
        get => _localImport;
        private set
        {
            if (!SetProperty(ref _localImport, value)) return;
            OnPropertyChanged(nameof(HasLocalImport));
        }
    }

    public bool HasLocalImport => _localImport is not null;

    /// <summary>首次显示下载页（或上次清单没拉到）时再拉一次版本清单。</summary>
    public void EnsureLoaded()
    {
        if (ManifestVersions.Count == 0) _ = LoadManifestAsync();
    }

    /// <summary>拖入 .zip / .mrpack：切到「整合包 → 本地文件」，进度直接显示在这个面板里。</summary>
    public void StartLocalImport(string path)
    {
        var pack = Categories.FirstOrDefault(c => c.Kind == "modpack");
        if (pack is not null) SelectedCategory = pack;
        if (SubTabs.Contains("本地文件")) SelectedSubTab = "本地文件";
        _ = ImportPackAsync(path);
    }

    private async Task ImportPackAsync(string path)
    {
        var fileName = Path.GetFileName(path);
        var task = Downloads.Begin($"安装整合包 {fileName}");
        LocalImport = task;

        var progress = new Progress<string>(text =>
        {
            task.Update(text);
            Status = text;
        });

        try
        {
            Status = $"正在安装 {fileName}…";
            var result = await _services.Modpacks.InstallFromFileAsync(path, null, progress, CancellationToken.None);
            _services.NotifyInstalledVersionsChanged();
            task.Complete($"安装完成：{result.VersionId}（{result.FilesInstalled} 个文件）");
            Status = $"整合包安装完成：{result.VersionId}";
        }
        catch (Exception ex)
        {
            _services.Log.Error($"整合包安装失败：{ex}");
            task.Fail("安装失败：" + ex.Message);
            Status = "整合包安装失败：" + ex.Message;
        }
    }

    /// <summary>二级菜单：按一级分类给出子选项。</summary>
    public string SelectedSubTab
    {
        get => _selectedSubTab;
        set
        {
            if (!SetProperty(ref _selectedSubTab, value)) return;
            if (IsVersionTab) ApplyVersionFilter();
            OnPropertyChanged(nameof(ShowLoader));
            OnPropertyChanged(nameof(ShowSearch));
            OnPropertyChanged(nameof(ShowSearchButton));
            OnPropertyChanged(nameof(ShowLocalHint));
            OnPropertyChanged(nameof(SubTabHint));
        }
    }

    public bool HasSubTabs => SubTabs.Count > 0;

    /// <summary>搜索框：版本列表实时过滤，内容分类按关键词搜索。</summary>
    public bool ShowSearch => IsVersionTab || SelectedSubTab != "热门";

    /// <summary>「搜索」按钮只给内容搜索用，版本列表是实时过滤。</summary>
    public bool ShowSearchButton => IsContentTab && SelectedSubTab != "热门";

    public bool ShowLocalHint => SelectedCategory.Kind == "modpack" && SelectedSubTab == "本地文件";

    public string SubTabHint => SelectedSubTab switch
    {
        "本地文件" => "把 .mrpack 或 CurseForge 整合包 zip 拖进窗口，安装进度就在这个面板里显示",
        "热门" => "按下载量排行的热门内容",
        _ => "",
    };

    private void RebuildSubTabs()
    {
        SubTabs.Clear();
        if (SelectedCategory.IsVersion)
        {
            // 正式版 / 快照版 / 愚人节版 分开列，另留一个「全部」。
            SubTabs.Add(VersionChannel.Release);
            SubTabs.Add(VersionChannel.Snapshot);
            SubTabs.Add(VersionChannel.AprilFools);
            SubTabs.Add("全部");
        }
        else if (SelectedCategory.Kind == "modpack")
        {
            SubTabs.Add("在线搜索");
            SubTabs.Add("本地文件");
        }
        else
        {
            SubTabs.Add("热门");
            SubTabs.Add("搜索");
        }

        SelectedSubTab = SubTabs.FirstOrDefault() ?? "";
        OnPropertyChanged(nameof(HasSubTabs));
        OnPropertyChanged(nameof(ShowSearch));
        OnPropertyChanged(nameof(ShowSearchButton));
        OnPropertyChanged(nameof(ShowLoader));
    }

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
            RebuildSubTabs();
            // 模组列表要按加载器筛：一进模组分类就把加载器定到 Fabric（原版模组基本不存在）。
            if (value.Kind == "mod" && SelectedLoader == "原版" && AvailableLoaders.Contains("Fabric"))
                SelectedLoader = "Fabric";
            OnPropertyChanged(nameof(InstallActionText));
            Status = value.IsVersion ? "选择版本后安装" : "加载热门内容…";
            if (value.IsVersion) _ = LoadManifestAsync();
            else _ = LoadFeaturedAsync();
        }
    }

    /// <summary>整合包用「安装整合包」按钮，其它内容仍是下载到版本目录。</summary>
    public string InstallActionText => SelectedCategory.Kind == "modpack" ? "安装整合包" : "下载到版本目录";

    public bool IsVersionTab => SelectedCategory.IsVersion;
    public bool IsContentTab => !SelectedCategory.IsVersion;

    /// <summary>加载器下拉：版本安装永远显示（默认「原版」），模组下载需要知道目标加载器。</summary>
    public bool ShowLoader => IsVersionTab || SelectedCategory.Kind == "mod";

    public string? SelectedGameVersion
    {
        get => _selectedGameVersion;
        set
        {
            if (!SetProperty(ref _selectedGameVersion, value)) return;
            InstallVersionCommand.RaiseCanExecuteChanged();
            RebuildAvailableLoaders();
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
            if (IsContentTab && SelectedCategory.Kind == "mod") _ = LoadFeaturedAsync();
        }
    }

    public LoaderVersion? SelectedLoaderVersion
    {
        get => _selectedLoaderVersion;
        set => SetProperty(ref _selectedLoaderVersion, value);
    }

    public ContentItemViewModel? SelectedProject
    {
        get => _selectedProject;
        set
        {
            if (!SetProperty(ref _selectedProject, value)) return;
            InstallContentCommand.RaiseCanExecuteChanged();
            _ = LoadSelectedDetailsAsync(value);
        }
    }

    /// <summary>选中项目的大图（优先第一张预览图，没有就用图标）。</summary>
    public Bitmap? SelectedBanner
    {
        get => _selectedBanner;
        private set
        {
            if (!SetProperty(ref _selectedBanner, value)) return;
            OnPropertyChanged(nameof(HasBanner));
        }
    }

    public bool HasBanner => _selectedBanner is not null;

    public bool HasGallery => Gallery.Count > 0;

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

    // ---------------- 版本清单 / 加载器 ----------------

    private async Task LoadManifestAsync()
    {
        if (_manifestLoading) return;
        _manifestLoading = true;
        try
        {
            Ui(() => Status = "加载版本清单…");
            var manifest = await _services.Manifests.GetAsync();
            Ui(() =>
            {
                var previous = SelectedGameVersion;
                ManifestVersions.Clear();
                FilteredVersions.Clear();
                _versionChannels.Clear();
                foreach (var entry in manifest.Versions)
                {
                    ManifestVersions.Add(entry.Id);
                    _versionChannels[entry.Id] = VersionChannel.Classify(entry.Id, entry.Type);
                }

                ApplyVersionFilter(keepSelection: false);
                var fallback = previous is not null && FilteredVersions.Contains(previous)
                    ? previous
                    : FilteredVersions.FirstOrDefault() ?? ManifestVersions.FirstOrDefault();
                SelectedGameVersion = fallback;
                Status = $"共 {ManifestVersions.Count} 个版本";
                if (IsVersionTab && Search.Trim().Length > 0) ApplyVersionFilter();
            });
        }
        catch (Exception ex)
        {
            Ui(() => Status = "清单加载失败：" + ex.Message);
        }
        finally
        {
            _manifestLoading = false;
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

        Ui(() =>
        {
            LoaderVersions.Clear();
            SelectedLoaderVersion = null;
        });
        if (CurrentLoaderType is not { } type || SelectedGameVersion is not { } mc) return;

        try
        {
            Ui(() => Status = $"加载 {SelectedLoader} 版本列表…");
            // 快速连点下拉框时只保留最后一次请求。
            await Task.Delay(250, token);
            var list = await _loaders.ListAsync(type, mc, token);
            if (token.IsCancellationRequested) return;
            Ui(() =>
            {
                LoaderVersions.Clear();
                foreach (var item in list) LoaderVersions.Add(item);
                SelectedLoaderVersion = LoaderVersions.FirstOrDefault();
                Status = LoaderVersions.Count > 0
                    ? $"{SelectedLoader} 共 {LoaderVersions.Count} 个构建"
                    : $"{mc} 没有可用的 {SelectedLoader} 版本，低版本一般只支持原版或 Forge";
            });
        }
        catch (OperationCanceledException)
        {
            Ui(() =>
            {
                if (!token.IsCancellationRequested) Status = $"{SelectedLoader} 列表加载超时";
            });
        }
        catch (Exception ex)
        {
            Ui(() =>
            {
                if (!token.IsCancellationRequested) Status = $"{SelectedLoader} 列表加载失败：" + ex.Message;
            });
        }
    }

    /// <summary>按当前游戏版本决定可选加载器：Fabric/Quilt 需要 1.14+，NeoForge 需要 1.20.1+。</summary>
    private void RebuildAvailableLoaders()
    {
        var wanted = new List<string> { "原版" };
        if (ParseMinecraftVersion(SelectedGameVersion) is { } version)
        {
            if (version >= new Version(1, 14)) { wanted.Add("Fabric"); wanted.Add("Quilt"); }
            wanted.Add("Forge");
            if (version >= new Version(1, 20, 1)) wanted.Add("NeoForge");
        }
        else
        {
            wanted.AddRange(["Fabric", "Quilt", "Forge", "NeoForge"]);
        }

        for (var i = AvailableLoaders.Count - 1; i >= 0; i--)
            if (!wanted.Contains(AvailableLoaders[i])) AvailableLoaders.RemoveAt(i);

        for (var i = 0; i < wanted.Count; i++)
        {
            if (i >= AvailableLoaders.Count) AvailableLoaders.Add(wanted[i]);
            else if (AvailableLoaders[i] != wanted[i]) AvailableLoaders.Insert(i, wanted[i]);
        }

        if (!wanted.Contains(SelectedLoader)) SelectedLoader = "原版";
    }

    /// <summary>把 "1.20.1" / "1.21.5-pre1" 解析成 Version；解析失败返回 null。</summary>
    private static Version? ParseMinecraftVersion(string? id)
    {
        if (string.IsNullOrWhiteSpace(id)) return null;
        var match = System.Text.RegularExpressions.Regex.Match(id, @"^(\d+)\.(\d+)(?:\.(\d+))?");
        if (!match.Success) return null;
        var major = int.Parse(match.Groups[1].Value);
        var minor = int.Parse(match.Groups[2].Value);
        var patch = match.Groups[3].Success ? int.Parse(match.Groups[3].Value) : 0;
        return new Version(major, minor, patch);
    }

    private void ApplyVersionFilter(bool keepSelection = true)
    {
        var keyword = Search.Trim();
        var channel = IsVersionTab ? SelectedSubTab : "全部";
        var previous = keepSelection ? SelectedGameVersion : null;
        FilteredVersions.Clear();
        foreach (var id in ManifestVersions)
        {
            if (keyword.Length > 0 && !id.Contains(keyword, StringComparison.OrdinalIgnoreCase)) continue;
            if (!string.IsNullOrEmpty(channel) && channel != "全部")
            {
                var value = _versionChannels.TryGetValue(id, out var v) ? v : VersionChannel.Legacy;
                if (!string.Equals(value, channel, StringComparison.Ordinal)) continue;
            }
            FilteredVersions.Add(id);
        }

        var channelText = string.IsNullOrEmpty(channel) || channel == "全部" ? "" : channel + " ";
        Status = keyword.Length == 0
            ? $"{channelText}共 {FilteredVersions.Count} 个版本"
            : $"{channelText}匹配 {FilteredVersions.Count} / {ManifestVersions.Count} 个版本";

        SelectedGameVersion = previous is not null && FilteredVersions.Contains(previous)
            ? previous
            : FilteredVersions.FirstOrDefault();
    }

    // ---------------- 内容浏览 ----------------

    private ContentKind CurrentContentKind => SelectedCategory.Kind switch
    {
        "mod" => ContentKind.Mod,
        "modpack" => ContentKind.Modpack,
        "shader" => ContentKind.Shader,
        "resourcepack" => ContentKind.ResourcePack,
        _ => ContentKind.DataPack
    };

    private void AddResult(ContentProject project)
    {
        var item = new ContentItemViewModel(project);
        item.LoadIcon(_services.Images);
        Results.Add(item);
    }

    /// <summary>选中项目后拉详情：图标 + 预览图，横向展示出来。</summary>
    private async Task LoadSelectedDetailsAsync(ContentItemViewModel? item)
    {
        Gallery.Clear();
        OnPropertyChanged(nameof(HasGallery));
        SelectedBanner = null;
        if (item is null) return;

        if (item.Icon is not null) SelectedBanner = item.Icon;

        try
        {
            var details = await _services.Modrinth.GetProjectAsync(item.Id, CancellationToken.None);
            if (!ReferenceEquals(SelectedProject, item)) return;

            var first = true;
            foreach (var url in details.Gallery.Take(8))
            {
                var image = await _services.Images.GetAsync(url);
                if (image is null) continue;
                Ui(() => Gallery.Add(image));
                if (first)
                {
                    var banner = image;
                    Ui(() => SelectedBanner = banner);
                    first = false;
                }
            }

            if (first && details.IconUrl is { } iconUrl)
            {
                var icon = await _services.Images.GetAsync(iconUrl);
                if (icon is not null) Ui(() => SelectedBanner = icon);
            }

            Ui(() => OnPropertyChanged(nameof(HasGallery)));
        }
        catch
        {
            // 详情拿不到就只显示图标，不影响下载。
        }
    }

    private async Task LoadFeaturedAsync()
    {
        if (IsVersionTab) return;
        _searchCts?.Cancel();
        _searchCts?.Dispose();
        _searchCts = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        var token = _searchCts.Token;

        IsBusy = true;
        try
        {
            var kind = CurrentContentKind;
            var loader = kind == ContentKind.Mod && SelectedLoader != "原版" ? SelectedLoader.ToLowerInvariant() : null;
            var list = await _services.Modrinth.FeaturedAsync(kind, SelectedGameVersion, loader, 30, token);
            if (token.IsCancellationRequested) return;
            Ui(() =>
            {
                Results.Clear();
                foreach (var item in list) AddResult(item);
                Status = list.Count == 0 ? "暂无热门内容" : $"热门 {list.Count} 个（按下载量），输入关键词可精确搜索";
            });
        }
        catch (OperationCanceledException)
        {
            Ui(() => Status = "热门内容加载超时");
        }
        catch (Exception ex)
        {
            Ui(() =>
            {
                if (!token.IsCancellationRequested) Status = "热门内容加载失败：" + ex.Message;
            });
        }
        finally
        {
            Ui(() => IsBusy = false);
        }
    }

    private async Task SearchAsync()
    {
        if (IsVersionTab)
        {
            ApplyVersionFilter();
            return;
        }

        if (string.IsNullOrWhiteSpace(Search))
        {
            await LoadFeaturedAsync();
            return;
        }

        _searchCts?.Cancel();
        _searchCts?.Dispose();
        _searchCts = new CancellationTokenSource(TimeSpan.FromSeconds(60));
        var token = _searchCts.Token;

        IsBusy = true;
        try
        {
            Ui(() => Status = "搜索中…");
            var kind = CurrentContentKind;
            var loader = kind == ContentKind.Mod && SelectedLoader != "原版" ? SelectedLoader.ToLowerInvariant() : null;
            var list = await _services.Modrinth.SearchAsync(kind, Search, SelectedGameVersion, loader, 30, token);
            Ui(() =>
            {
                Results.Clear();
                foreach (var item in list) AddResult(item);
                Status = list.Count == 0 ? "没有找到匹配内容" : $"找到 {list.Count} 个结果";
            });
        }
        catch (OperationCanceledException)
        {
            Ui(() => Status = "搜索超时");
        }
        catch (Exception ex)
        {
            Ui(() =>
            {
                if (!token.IsCancellationRequested) Status = "搜索失败：" + ex.Message;
            });
        }
        finally
        {
            Ui(() => IsBusy = false);
        }
    }

    // ---------------- 安装 / 下载 ----------------

    private async Task InstallVersionAsync()
    {
        if (SelectedGameVersion is not { } mc) return;
        IsBusy = true;
        Progress = 0;
        var task = Downloads.Begin($"安装 {mc}" + (CurrentLoaderType is null ? "" : $" + {SelectedLoader}"));
        try
        {
            Status = $"安装原版 {mc}…";
            task.Update($"开始安装 {mc}");
            await _services.Installer.InstallAsync(mc, new Progress<InstallProgress>(p => Ui(() =>
            {
                Progress = p.Total > 0 ? (double)p.Completed / p.Total : 0;
                Status = p.Stage;
                task.Update(p.Stage, p.Completed, p.Total, p.DownloadedBytes, p.TotalBytes);
            })), CancellationToken.None, asDependency: CurrentLoaderType is not null);

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
                task.Update($"安装 {SelectedLoader} {loaderVersion}");
                var id = await _loaders.InstallAsync(type, mc, loaderVersion,
                    new Progress<string>(s => Ui(() => { Status = s; task.Update(s); })), CancellationToken.None);
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
            Downloads.Finish(task, "安装完成");
        }
        catch (Exception ex)
        {
            Status = "安装失败：" + ex.Message;
            Downloads.Fail(task, "安装失败：" + ex.Message);
        }
        finally
        {
            IsBusy = false;
        }
    }

    private async Task InstallContentAsync()
    {
        if (SelectedProject is not { } item || SelectedGameVersion is not { } mc) return;
        var project = item.Project;
        IsBusy = true;
        Progress = 0;
        var task = Downloads.Begin($"下载 {item.Title}");
        try
        {
            var kind = CurrentContentKind;
            // 整合包会自己把原版和加载器装好；模组/光影等需要版本已存在。
            if (kind != ContentKind.Modpack && !Directory.Exists(_services.Paths.VersionDir(mc)))
            {
                Status = $"请先安装 {mc} 再下载内容";
                Downloads.Fail(task, $"请先安装 {mc}");
                return;
            }

            var gameDir = _services.Paths.GameDir(mc);
            var loader = kind == ContentKind.Mod && SelectedLoader != "原版" ? SelectedLoader.ToLowerInvariant() : null;

            if (kind == ContentKind.Modpack)
            {
                Status = $"安装整合包 {project.Title}…";
                var progress = new Progress<string>(s => Ui(() => { Status = s; task.Update(s); }));
                var result = await _services.Modpacks.InstallAsync(project.Slug, mc, loader, progress, CancellationToken.None);
                _services.NotifyInstalledVersionsChanged();
                Status = $"已安装整合包：{result.VersionId}（{result.FilesInstalled} 个文件），去「游戏版本」里启动";
                Downloads.Finish(task, $"完成：{result.FilesInstalled} 个文件，{result.OverridesInstalled} 个配置");
                Ui(() => SelectedGameVersion = mc);
                return;
            }

            Status = $"下载 {project.Title}…";
            task.Update($"下载 {project.Title}");
            var path = await _services.Modrinth.InstallAsync(kind, project.Id, mc, loader, gameDir,
                new Progress<double>(v => Ui(() => { Progress = v; task.Update(string.Empty, null, null, null, null); })), CancellationToken.None);
            Status = "已下载：" + path;
            Downloads.Finish(task, "下载完成：" + path);
        }
        catch (Exception ex)
        {
            Status = "下载失败：" + ex.Message;
            Downloads.Fail(task, "下载失败：" + ex.Message);
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>把界面更新切回 UI 线程（Avalonia 的绑定不会自动跨线程）。</summary>
    private static void Ui(Action action)
    {
        if (Dispatcher.UIThread.CheckAccess()) action();
        else Dispatcher.UIThread.Post(action);
    }
}
