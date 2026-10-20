using System.IO;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;

namespace DLP.App.ViewModels;

public sealed class InstancesViewModel : ObservableObject
{
    private readonly AppServices _services;
    private string _newName = "";
    private string? _selected;
    private string _status = "";

    public InstancesViewModel(AppServices services)
    {
        _services = services;
        Instances = [];
        CreateCommand = new RelayCommand(Create);
        OpenCommand = new RelayCommand(Open, () => Selected is not null);
        RefreshCommand = new RelayCommand(Refresh);
        Refresh();
    }

    public ObservableCollection<string> Instances { get; }
    public RelayCommand CreateCommand { get; }
    public RelayCommand OpenCommand { get; }
    public RelayCommand RefreshCommand { get; }

    public string NewName
    {
        get => _newName;
        set => SetProperty(ref _newName, value);
    }

    public string? Selected
    {
        get => _selected;
        set { if (SetProperty(ref _selected, value)) OpenCommand.RaiseCanExecuteChanged(); }
    }

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public string DataRoot => _services.Paths.Root;

    private void Refresh()
    {
        var previous = Selected;
        Instances.Clear();
        foreach (var instance in _services.GetInstances()) Instances.Add(instance);
        Selected = previous is not null && Instances.Contains(previous) ? previous : Instances.FirstOrDefault();
    }

    private void Create()
    {
        var name = NewName.Trim();
        if (name.Length == 0) { Status = "请输入实例名称"; return; }
        if (name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0 || name.Contains("..")) { Status = "名称含有非法字符"; return; }

        var dir = _services.Paths.InstanceDir(name);
        if (Directory.Exists(dir)) { Status = "实例已存在"; return; }

        Directory.CreateDirectory(dir);
        foreach (var sub in new[] { "minecraft/mods", "minecraft/saves", "minecraft/resourcepacks", "minecraft/shaderpacks", "minecraft/logs" })
            Directory.CreateDirectory(Path.Combine(dir, sub));
        File.WriteAllText(Path.Combine(dir, "instance.json"), $"{{\n  \"name\": \"{name}\"\n}}");

        NewName = "";
        Refresh();
        Selected = name;
        Status = $"已创建实例 {name}";
    }

    private void Open()
    {
        if (Selected is not { } name) return;
        Process.Start(new ProcessStartInfo(_services.Paths.InstanceGameDir(name)) { UseShellExecute = true });
    }
}