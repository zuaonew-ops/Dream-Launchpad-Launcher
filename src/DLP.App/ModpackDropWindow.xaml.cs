// 作者：zuao
using System.ComponentModel;
using System.IO;
using System.Runtime.CompilerServices;
using System.Windows;
using DLP.App.Services;

namespace DLP.App;

/// <summary>拖入 .mrpack / .zip 后显示安装进度的小窗口。</summary>
public partial class ModpackDropWindow : Window, INotifyPropertyChanged
{
    private readonly AppServices _services;
    private readonly string _mrpackPath;
    private readonly CancellationTokenSource _cts = new();
    private string _status = "准备安装…";
    private bool _isInstalling = true;

    public ModpackDropWindow(AppServices services, string mrpackPath)
    {
        InitializeComponent();
        _services = services;
        _mrpackPath = mrpackPath;
        DataContext = this;
        Loaded += async (_, _) => await InstallAsync();
        Closed += (_, _) => _cts.Cancel();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public string FileName => Path.GetFileName(_mrpackPath) ?? _mrpackPath;

    public string Status
    {
        get => _status;
        private set { _status = value; OnPropertyChanged(); }
    }

    /// <summary>安装过程中禁用关闭按钮，完成或失败后启用。</summary>
    public bool CanClose => !_isInstalling;

    private async Task InstallAsync()
    {
        try
        {
            var progress = new Progress<string>(text => Status = text);
            var result = await _services.Modpacks.InstallFromFileAsync(_mrpackPath, null, progress, _cts.Token);
            _services.NotifyInstalledVersionsChanged();
            Status = $"安装完成：{result.VersionId}（{result.FilesInstalled} 个文件）";
        }
        catch (OperationCanceledException)
        {
            Status = "已取消";
        }
        catch (Exception ex)
        {
            _services.Log.Error($"拖拽安装整合包失败：{ex}");
            Status = "安装失败：" + ex.Message;
        }
        finally
        {
            _isInstalling = false;
            OnPropertyChanged(nameof(CanClose));
        }
    }

    private void Close_Click(object sender, RoutedEventArgs e) => Close();

    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
