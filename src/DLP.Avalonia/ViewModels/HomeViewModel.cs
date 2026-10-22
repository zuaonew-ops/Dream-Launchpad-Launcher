using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Avalonia.Services;
using DLP.Core.Java;
using DLP.Core.Launch;

namespace DLP.Avalonia.ViewModels;

public sealed class HomeViewModel : ObservableObject
{
    private readonly AppServices _services;
    private string? _selectedVersion;
    private string _username;
    private int _memoryMb;
    private string _status = "就绪";
    private bool _isBusy;

    public int MaxMemoryMb { get; } = Math.Clamp(
        (int)(GC.GetGCMemoryInfo().TotalAvailableMemoryBytes / 1024 / 1024), 1024, 65536);

    public HomeViewModel(AppServices services)
    {
        _services = services;
        _username = services.Settings.Username;
        _memoryMb = Math.Clamp(services.Settings.MemoryMb, 512, MaxMemoryMb);
        Accounts = new AccountsViewModel(services);

        InstalledVersions = [];
        LaunchCommand = new AsyncRelayCommand(LaunchAsync, () => !IsBusy && SelectedVersion is not null);
        RefreshCommand = new RelayCommand(Refresh);
        OpenFolderCommand = new RelayCommand(OpenGameFolder);

        _services.Accounts.Changed += () =>
        {
            OnPropertyChanged(nameof(AccountName));
            OnPropertyChanged(nameof(AccountKind));
        };

        Refresh();
    }

    public ObservableCollection<string> InstalledVersions { get; }

    /// <summary>首页内嵌的账号管理卡片。</summary>
    public AccountsViewModel Accounts { get; }
    public AsyncRelayCommand LaunchCommand { get; }
    public RelayCommand RefreshCommand { get; }
    public RelayCommand OpenFolderCommand { get; }

    public string? SelectedVersion
    {
        get => _selectedVersion;
        set
        {
            if (!SetProperty(ref _selectedVersion, value)) return;
            LaunchCommand.RaiseCanExecuteChanged();
            OnPropertyChanged(nameof(GameDir));
        }
    }

    public string AccountName => _services.Accounts.CurrentName;
    public string AccountKind => _services.Accounts.CurrentKind;
    public string Username { get => _username; set => SetProperty(ref _username, value); }
    public int MemoryMb { get => _memoryMb; set => SetProperty(ref _memoryMb, value); }
    public string Status { get => _status; set => SetProperty(ref _status, value); }

    public bool IsBusy
    {
        get => _isBusy;
        set { if (SetProperty(ref _isBusy, value)) LaunchCommand.RaiseCanExecuteChanged(); }
    }

    public bool HasVersions => InstalledVersions.Count > 0;
    public bool HasNoVersions => InstalledVersions.Count == 0;
    public int InstalledVersionCount => InstalledVersions.Count;
    public string DataRoot => _services.Paths.Root;
    public string GameDir => SelectedVersion is { } id ? _services.Paths.GameDir(id) : "";

    public void Refresh()
    {
        var previous = SelectedVersion;
        InstalledVersions.Clear();
        foreach (var version in _services.GetInstalledVersions()) InstalledVersions.Add(version);
        SelectedVersion = previous is not null && InstalledVersions.Contains(previous)
            ? previous
            : InstalledVersions.FirstOrDefault();

        OnPropertyChanged(nameof(HasVersions));
        OnPropertyChanged(nameof(HasNoVersions));
        OnPropertyChanged(nameof(InstalledVersionCount));
        LaunchCommand.RaiseCanExecuteChanged();
    }

    private void OpenGameFolder()
    {
        if (SelectedVersion is not { } id) return;
        var dir = _services.Paths.GameDir(id);
        foreach (var sub in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
            Directory.CreateDirectory(Path.Combine(dir, sub));
        Shell.Open(dir);
    }

    private async Task LaunchAsync()
    {
        if (SelectedVersion is not { } versionId) return;
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

            _services.Settings.Username = Username;
            _services.Settings.MemoryMb = MemoryMb;
            _services.Settings.Save();

            var gameDir = _services.Paths.GameDir(versionId);
            foreach (var dir in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
                Directory.CreateDirectory(Path.Combine(gameDir, dir));

            var options = new LaunchOptions
            {
                Account = _services.Accounts.GetLaunchAccount(),
                JavaPath = runtime.Path,
                GameDirectory = gameDir,
                MemoryMb = MemoryMb
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
            Status = ex.Message;
            _services.Log.Error($"启动失败：{ex.Message}");
        }
        finally
        {
            IsBusy = false;
        }
    }
}
