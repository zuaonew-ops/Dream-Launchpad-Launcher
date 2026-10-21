using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;

namespace DLP.App.ViewModels;

public sealed class VersionsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private string? _selected;
    private string _status = "";

    public VersionsViewModel(AppServices services)
    {
        _services = services;
        InstalledVersions = [];
        RefreshCommand = new RelayCommand(Refresh);
        OpenVersionFolderCommand = new RelayCommand(OpenVersionFolder);
        OpenGameFolderCommand = new RelayCommand(OpenGameFolder);
        DeleteCommand = new RelayCommand(Delete);

        _services.InstalledVersionsChanged += () => Application.Current.Dispatcher.Invoke(Refresh);
        Refresh();
    }

    public ObservableCollection<string> InstalledVersions { get; }
    public RelayCommand RefreshCommand { get; }
    public RelayCommand OpenVersionFolderCommand { get; }
    public RelayCommand OpenGameFolderCommand { get; }
    public RelayCommand DeleteCommand { get; }

    public string? Selected
    {
        get => _selected;
        set
        {
            if (!SetProperty(ref _selected, value)) return;
            OnPropertyChanged(nameof(VersionDir));
            OnPropertyChanged(nameof(GameDir));
            OnPropertyChanged(nameof(HasSelection));
        }
    }

    public bool HasSelection => Selected is not null;
    public string VersionsRoot => _services.Paths.Versions;
    public string VersionDir => Selected is { } id ? _services.Paths.VersionDir(id) : "";
    public string GameDir => Selected is { } id ? _services.Paths.GameDir(id) : "";

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public void Refresh()
    {
        var previous = Selected;
        InstalledVersions.Clear();
        foreach (var id in _services.GetInstalledVersions()) InstalledVersions.Add(id);
        Selected = previous is not null && InstalledVersions.Contains(previous)
            ? previous
            : InstalledVersions.FirstOrDefault();
        Status = InstalledVersions.Count == 0 ? "还没有安装版本，去「下载」页安装" : $"共 {InstalledVersions.Count} 个已安装版本";
    }

    private void OpenVersionFolder()
    {
        if (Selected is not { } id) return;
        Directory.CreateDirectory(_services.Paths.VersionDir(id));
        Process.Start(new ProcessStartInfo(_services.Paths.VersionDir(id)) { UseShellExecute = true });
    }

    private void OpenGameFolder()
    {
        if (Selected is not { } id) return;
        var dir = _services.Paths.GameDir(id);
        foreach (var sub in new[] { "mods", "saves", "resourcepacks", "shaderpacks", "logs" })
            Directory.CreateDirectory(Path.Combine(dir, sub));
        Process.Start(new ProcessStartInfo(dir) { UseShellExecute = true });
    }

    private void Delete()
    {
        if (Selected is not { } id) return;
        var result = MessageBox.Show($"确定删除版本「{id}」？\n版本目录与隔离游戏目录都会被删除。", "删除版本",
            MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (result != MessageBoxResult.Yes) return;
        try
        {
            var dir = _services.Paths.VersionDir(id);
            if (Directory.Exists(dir)) Directory.Delete(dir, recursive: true);
            var cleaned = _services.CleanupOrphanDependencies();
            Status = cleaned > 0 ? $"已删除 {id}（同时清理 {cleaned} 个依赖原版）" : $"已删除 {id}";
            _services.NotifyInstalledVersionsChanged();
        }
        catch (Exception ex)
        {
            Status = "删除失败：" + ex.Message;
        }
    }
}
