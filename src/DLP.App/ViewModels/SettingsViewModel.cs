using System.Collections.ObjectModel;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Core.Download;

namespace DLP.App.ViewModels;

public sealed class SettingsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private int _memoryMb;
    private string _username;
    private string _javaPath;
    private string _downloadSource;
    private string _clientId;
    private string _status = "";

    public SettingsViewModel(AppServices services)
    {
        _services = services;
        MaxMemoryMb = Math.Clamp((int)(GC.GetGCMemoryInfo().TotalAvailableMemoryBytes / 1024 / 1024), 1024, 65536);
        _memoryMb = Math.Clamp(services.Settings.MemoryMb, 512, MaxMemoryMb);
        _username = services.Settings.Username;
        _javaPath = services.Settings.JavaPath ?? "";
        _downloadSource = services.Settings.DownloadSource;
        _clientId = services.Settings.MicrosoftClientId ?? "";
        SaveCommand = new RelayCommand(Save);
        BrowseJavaCommand = new RelayCommand(BrowseJava);
        ScanJavaCommand = new RelayCommand(ScanJava);
        DetectedJava = [];
        ScanJava();
    }

    public RelayCommand SaveCommand { get; }
    public RelayCommand BrowseJavaCommand { get; }
    public RelayCommand ScanJavaCommand { get; }
    public ObservableCollection<string> DetectedJava { get; }
    public string[] Sources { get; } = ["bmclapi", "official"];

    public int MemoryMb
    {
        get => _memoryMb;
        set => SetProperty(ref _memoryMb, value);
    }

    public string Username
    {
        get => _username;
        set => SetProperty(ref _username, value);
    }

    public string JavaPath
    {
        get => _javaPath;
        set => SetProperty(ref _javaPath, value);
    }

    public string DownloadSource
    {
        get => _downloadSource;
        set => SetProperty(ref _downloadSource, value);
    }

    public string MicrosoftClientId
    {
        get => _clientId;
        set => SetProperty(ref _clientId, value);
    }

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public string DataRoot => _services.Paths.Root;
    public int MaxMemoryMb { get; }

    private void Save()
    {
        var settings = _services.Settings;
        settings.MemoryMb = Math.Clamp(MemoryMb, 512, MaxMemoryMb);
        settings.Username = string.IsNullOrWhiteSpace(Username) ? "Steve" : Username;
        settings.JavaPath = string.IsNullOrWhiteSpace(JavaPath) ? null : JavaPath;
        settings.DownloadSource = DownloadSource;
        settings.MicrosoftClientId = string.IsNullOrWhiteSpace(MicrosoftClientId) ? null : MicrosoftClientId.Trim();
        try
        {
            settings.Save();
            Mirrors.Enabled = DownloadSource != "official";
            if (_services.Accounts.Current?.Kind != "msa") _services.Accounts.SetOffline(settings.Username);
            Status = "已保存";
        }
        catch (Exception ex)
        {
            Status = "保存失败：" + ex.Message;
        }
    }

    private void BrowseJava()
    {
        var dialog = new Microsoft.Win32.OpenFileDialog
        {
            Filter = "java.exe|java.exe",
            Title = "选择 Java"
        };
        if (dialog.ShowDialog() == true) JavaPath = dialog.FileName;
    }

    private void ScanJava()
    {
        DetectedJava.Clear();
        foreach (var runtime in _services.Java.Scan())
            DetectedJava.Add($"Java {runtime.MajorVersion} · {(runtime.Is64Bit ? "64 位" : "32 位")} · {runtime.Path}");
        if (DetectedJava.Count == 0) DetectedJava.Add("未检测到 Java（启动时会自动下载）");
    }
}
