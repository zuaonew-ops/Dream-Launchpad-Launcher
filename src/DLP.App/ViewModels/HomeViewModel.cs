using System.IO;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Core.Java;
using DLP.Core.Launch;
using DLP.Core.Auth;

namespace DLP.App.ViewModels;

public sealed class HomeViewModel : ObservableObject
{
    private readonly AppServices _services;
    private string? _selectedInstance;
    private string? _selectedVersion;
    private string _username;
    private int _memoryMb;
    private string _status = "就绪";
    private bool _isBusy;
    private bool _isRunning;

    public HomeViewModel(AppServices services)
    {
        _services = services;
        _username = services.Settings.Username;
        _memoryMb = services.Settings.MemoryMb;

        Instances = [];
        InstalledVersions = [];
        LaunchCommand = new AsyncRelayCommand(LaunchAsync, () => !IsBusy && SelectedVersion is not null);
        RefreshCommand = new RelayCommand(Refresh);
        OpenFolderCommand = new RelayCommand(OpenInstanceFolder);

        _services.InstalledVersionsChanged += () => Application.Current.Dispatcher.Invoke(Refresh);
        _services.Log.Written += (level, line) =>
        {
            if (level == DLP.Core.Infrastructure.LogLevel.Error)
                Application.Current.Dispatcher.BeginInvoke(() => Status = line.Split("] ", 2).LastOrDefault() ?? line);
        };

        Refresh();
    }

    public ObservableCollection<string> Instances { get; }
    public ObservableCollection<string> InstalledVersions { get; }
    public AsyncRelayCommand LaunchCommand { get; }
    public RelayCommand RefreshCommand { get; }
    public RelayCommand OpenFolderCommand { get; }

    public string? SelectedInstance
    {
        get => _selectedInstance;
        set { if (SetProperty(ref _selectedInstance, value)) _services.Settings.LastInstance = value ?? "default"; }
    }

    public string? SelectedVersion
    {
        get => _selectedVersion;
        set { if (SetProperty(ref _selectedVersion, value)) LaunchCommand.RaiseCanExecuteChanged(); }
    }

    public string Username
    {
        get => _username;
        set => SetProperty(ref _username, value);
    }

    public int MemoryMb
    {
        get => _memoryMb;
        set => SetProperty(ref _memoryMb, value);
    }

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public bool IsBusy
    {
        get => _isBusy;
        set { if (SetProperty(ref _isBusy, value)) LaunchCommand.RaiseCanExecuteChanged(); }
    }

    public bool IsRunning
    {
        get => _isRunning;
        set => SetProperty(ref _isRunning, value);
    }

    public bool HasVersions => InstalledVersions.Count > 0;
    public bool HasNoVersions => InstalledVersions.Count == 0;
    public int InstalledVersionCount => InstalledVersions.Count;
    public int InstanceCount => Instances.Count;
    public string DataRoot => _services.Paths.Root;

    public void Refresh()
    {
        var instances = _services.GetInstances();
        var versions = _services.GetInstalledVersions();

        var previousInstance = SelectedInstance;
        var previousVersion = SelectedVersion;

        Instances.Clear();
        foreach (var instance in instances) Instances.Add(instance);
        if (Instances.Count == 0)
        {
            var defaultName = "default";
            Directory.CreateDirectory(_services.Paths.InstanceGameDir(defaultName));
            Instances.Add(defaultName);
        }
        SelectedInstance = previousInstance ?? _services.Settings.LastInstance;
        if (!Instances.Contains(SelectedInstance ?? "")) SelectedInstance = Instances[0];

        InstalledVersions.Clear();
        foreach (var version in versions) InstalledVersions.Add(version);
        SelectedVersion = previousVersion ?? InstalledVersions.FirstOrDefault();

        OnPropertyChanged(nameof(HasVersions));
        OnPropertyChanged(nameof(HasNoVersions));
        OnPropertyChanged(nameof(InstalledVersionCount));
        OnPropertyChanged(nameof(InstanceCount));
        LaunchCommand.RaiseCanExecuteChanged();
    }

    private void OpenInstanceFolder()
    {
        var dir = _services.Paths.InstanceGameDir(SelectedInstance ?? "default");
        Directory.CreateDirectory(dir);
        Process.Start(new ProcessStartInfo(dir) { UseShellExecute = true });
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
            JavaRuntime? runtime = null;
            if (_services.Settings.JavaPath is { } manual && File.Exists(manual))
                runtime = new JavaRuntime(manual, 0, true, "manual", manual);
            else
                runtime = _services.Java.Select(required);

            if (runtime is null)
            {
                Status = $"未找到 Java {required}，正在下载 Mojang 运行时…";
                runtime = await _services.Java.EnsureAsync(version);
            }

            _services.Settings.Username = Username;
            _services.Settings.MemoryMb = MemoryMb;
            _services.Settings.Save();

            var instance = SelectedInstance ?? "default";
            var gameDir = _services.Paths.InstanceGameDir(instance);
            foreach (var dir in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
                Directory.CreateDirectory(Path.Combine(gameDir, dir));

            var options = new LaunchOptions
            {
                Account = LaunchAccount.Offline(string.IsNullOrWhiteSpace(Username) ? "Steve" : Username),
                JavaPath = runtime.Path,
                GameDirectory = gameDir,
                MemoryMb = MemoryMb
            };

            var process = _services.Launcher.BuildProcess(version, options);
            process.Start();
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();

            IsRunning = true;
            Status = $"游戏运行中（PID {process.Id}，Java {runtime.MajorVersion}）";
            await process.WaitForExitAsync();
            Status = $"游戏已退出（退出码 {process.ExitCode}）";
        }
        catch (Exception ex)
        {
            Status = "启动失败：" + ex.Message;
        }
        finally
        {
            IsRunning = false;
            IsBusy = false;
        }
    }
}