using System.Collections.ObjectModel;
using System.IO;
using Avalonia.Threading;
using DLP.App.Mvvm;
using DLP.Avalonia.Services;
using DLP.Core.Java;
using DLP.Core.Launch;

namespace DLP.Avalonia.ViewModels;

/// <summary>「游戏版本」列表里的一项。</summary>
public sealed class VersionItem(string id)
{
    public string Id { get; } = id;
}

/// <summary>游戏版本页（Avalonia）：已安装版本列表、启动、打开目录、删除（页内二次确认）。</summary>
public sealed class VersionsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private VersionItem? _selected;
    private string _status = "";
    private bool _isBusy;
    private string? _pendingDelete;

    public VersionsViewModel(AppServices services)
    {
        _services = services;
        InstalledVersions = [];
        RefreshCommand = new RelayCommand(Refresh);
        LaunchCommand = new AsyncRelayCommand(LaunchAsync, () => !IsBusy && Selected is not null);
        OpenVersionFolderCommand = new RelayCommand(OpenVersionFolder, () => Selected is not null);
        OpenGameFolderCommand = new RelayCommand(OpenGameFolder, () => Selected is not null);
        RequestDeleteCommand = new RelayCommand(RequestDelete, () => Selected is not null && !IsBusy);
        ConfirmDeleteCommand = new RelayCommand(ConfirmDelete);
        CancelDeleteCommand = new RelayCommand(CancelDelete);

        // 安装或删除版本后刷新列表；事件可能来自后台线程，统一回到 UI 线程。
        _services.InstalledVersionsChanged += () => Dispatcher.UIThread.Post(Refresh);

        Refresh();
    }

    public ObservableCollection<VersionItem> InstalledVersions { get; }
    public RelayCommand RefreshCommand { get; }
    public AsyncRelayCommand LaunchCommand { get; }
    public RelayCommand OpenVersionFolderCommand { get; }
    public RelayCommand OpenGameFolderCommand { get; }
    public RelayCommand RequestDeleteCommand { get; }
    public RelayCommand ConfirmDeleteCommand { get; }
    public RelayCommand CancelDeleteCommand { get; }

    public VersionItem? Selected
    {
        get => _selected;
        set
        {
            if (!SetProperty(ref _selected, value)) return;
            OnPropertyChanged(nameof(VersionTitle));
            OnPropertyChanged(nameof(VersionId));
            OnPropertyChanged(nameof(VersionDir));
            OnPropertyChanged(nameof(GameDir));
            OnPropertyChanged(nameof(HasSelection));
            RaiseCommandStates();
            if (_pendingDelete is not null && _pendingDelete != value?.Id) CancelDelete();
        }
    }

    public bool HasSelection => Selected is not null;
    public bool HasVersions => InstalledVersions.Count > 0;
    public bool HasNoVersions => InstalledVersions.Count == 0;
    public int InstalledCount => InstalledVersions.Count;
    public string VersionsRoot => _services.Paths.Versions;
    public string VersionTitle => Selected?.Id ?? "未选择版本";
    public string VersionId => Selected?.Id ?? "";
    public string VersionDir => Selected is { } item ? _services.Paths.VersionDir(item.Id) : "";
    public string GameDir => Selected is { } item ? _services.Paths.GameDir(item.Id) : "";

    public string Status { get => _status; set => SetProperty(ref _status, value); }

    public bool IsBusy
    {
        get => _isBusy;
        set
        {
            if (SetProperty(ref _isBusy, value)) RaiseCommandStates();
        }
    }

    public bool IsConfirmingDelete => _pendingDelete is not null;
    public string PendingDeleteId => _pendingDelete ?? "";
    public string ConfirmMessage => _pendingDelete is { } id
        ? $"确定删除版本「{id}」？版本目录与隔离游戏目录都会被删除。"
        : "";

    /// <summary>重新扫描 versions 目录，保持当前选中项（若仍存在）。</summary>
    public void Refresh()
    {
        if (!Dispatcher.UIThread.CheckAccess())
        {
            Dispatcher.UIThread.Post(Refresh);
            return;
        }

        var previous = Selected?.Id;
        InstalledVersions.Clear();
        foreach (var id in _services.GetInstalledVersions()) InstalledVersions.Add(new VersionItem(id));
        Selected = previous is not null ? InstalledVersions.FirstOrDefault(v => v.Id == previous) : null;
        Selected ??= InstalledVersions.FirstOrDefault();

        OnPropertyChanged(nameof(HasVersions));
        OnPropertyChanged(nameof(HasNoVersions));
        OnPropertyChanged(nameof(InstalledCount));
        RaiseCommandStates();
        Status = InstalledVersions.Count == 0
            ? "还没有安装版本，去「下载」页安装"
            : $"共 {InstalledVersions.Count} 个已安装版本";
    }

    private void RaiseCommandStates()
    {
        LaunchCommand.RaiseCanExecuteChanged();
        OpenVersionFolderCommand.RaiseCanExecuteChanged();
        OpenGameFolderCommand.RaiseCanExecuteChanged();
        RequestDeleteCommand.RaiseCanExecuteChanged();
    }

    private void OpenVersionFolder()
    {
        if (Selected is not { } item) return;
        var dir = _services.Paths.VersionDir(item.Id);
        Directory.CreateDirectory(dir);
        Shell.Open(dir);
        Status = $"已打开版本目录：{dir}";
    }

    private void OpenGameFolder()
    {
        if (Selected is not { } item) return;
        var dir = _services.Paths.GameDir(item.Id);
        foreach (var sub in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
            Directory.CreateDirectory(Path.Combine(dir, sub));
        Shell.Open(dir);
        Status = $"已打开游戏目录：{dir}";
    }

    private void RequestDelete()
    {
        if (Selected is not { } item) return;
        _pendingDelete = item.Id;
        NotifyDeleteState();
        Status = $"待确认删除：{item.Id}";
    }

    private void CancelDelete()
    {
        if (_pendingDelete is null) return;
        _pendingDelete = null;
        NotifyDeleteState();
        Status = "已取消删除";
    }

    private void ConfirmDelete()
    {
        if (_pendingDelete is not { } id) return;

        try
        {
            var dir = _services.Paths.VersionDir(id);
            if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
            _pendingDelete = null;
            NotifyDeleteState();
            _services.Log.Info($"删除版本 {id}");
            Refresh();
            Status = $"已删除 {id}";
        }
        catch (Exception ex)
        {
            Status = "删除失败：" + ex.Message;
            _services.Log.Error($"删除版本失败：{ex.Message}");
        }
    }

    private void NotifyDeleteState()
    {
        OnPropertyChanged(nameof(IsConfirmingDelete));
        OnPropertyChanged(nameof(PendingDeleteId));
        OnPropertyChanged(nameof(ConfirmMessage));
    }

    private async Task LaunchAsync()
    {
        if (Selected is not { } item) return;
        var versionId = item.Id;
        _services.Log.Info($"开始启动 {versionId}");

        IsBusy = true;
        Status = "准备启动…";
        try
        {
            var version = await _services.Versions.ResolveAsync(versionId);
            var required = version.JavaVersion?.MajorVersion ?? 8;
            JavaRuntime? runtime = _services.Settings.JavaPath is { } manual && File.Exists(manual)
                ? new JavaRuntime(manual, 0, true, "manual", manual)
                : _services.Java.Select(required);

            if (runtime is null)
            {
                Status = $"未找到 Java {required}，正在下载运行时…";
                runtime = await _services.Java.EnsureAsync(version);
            }

            if (runtime is null)
            {
                Status = $"没有可用的 Java {required}";
                return;
            }

            var gameDir = _services.Paths.GameDir(versionId);
            foreach (var dir in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
                Directory.CreateDirectory(Path.Combine(gameDir, dir));

            var memory = Math.Clamp(_services.Settings.MemoryMb, 512, 65536);
            var options = new LaunchOptions
            {
                Account = _services.Accounts.GetLaunchAccount(),
                JavaPath = runtime.Path,
                GameDirectory = gameDir,
                MemoryMb = memory
            };

            Status = "启动中…";
            var process = _services.Launcher.BuildProcess(version, options);
            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();

            Status = $"游戏运行中（PID {process.Id}，Java {runtime.MajorVersion}）";
            _services.Log.Info($"启动成功：{versionId}（PID {process.Id}）");
            await process.WaitForExitAsync();
            Status = $"游戏已退出（退出码 {process.ExitCode}）";
            Refresh();
        }
        catch (Exception ex)
        {
            Status = "启动失败：" + ex.Message;
            _services.Log.Error($"启动失败：{ex.Message}");
        }
        finally
        {
            IsBusy = false;
        }
    }
}
