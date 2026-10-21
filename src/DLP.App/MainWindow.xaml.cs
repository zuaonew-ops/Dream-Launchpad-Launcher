using System.ComponentModel;
using System.IO;
using System.Windows;
using System.Windows.Media.Animation;
using DLP.App.Services;
using DLP.App.ViewModels;

namespace DLP.App;

public partial class MainWindow : Window
{
    private DownloadMonitor? _monitor;

    public MainWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) => AttachMonitor();
        Loaded += (_, _) =>
        {
            AttachMonitor();
            if (Environment.GetCommandLineArgs().Contains("--demo-download")) _ = RunDemoDownloadAsync();
        };
    }

    private void AttachMonitor()
    {
        if (DataContext is not MainViewModel vm) return;
        if (ReferenceEquals(_monitor, vm.Downloads)) return;
        if (_monitor is not null) _monitor.PropertyChanged -= Monitor_PropertyChanged;
        _monitor = vm.Downloads;
        _monitor.PropertyChanged += Monitor_PropertyChanged;
        if (_monitor.HasActive) ShowTaskPopup();
    }

    private void Monitor_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(DownloadMonitor.HasActive) || _monitor is null) return;
        if (_monitor.HasActive) ShowTaskPopup();
        else _ = HideTaskPopupLaterAsync();
    }

    private async Task HideTaskPopupLaterAsync()
    {
        await Task.Delay(2600);
        if (_monitor is { HasActive: true }) return;
        HideTaskPopup();
        _monitor?.ClearFinished();
    }

    private void ShowTaskPopup()
    {
        if (TaskPopup.Visibility != Visibility.Visible)
        {
            TaskPopup.Visibility = Visibility.Visible;
            TaskPopup.BeginAnimation(OpacityProperty, new DoubleAnimation(0, 1, TimeSpan.FromMilliseconds(220)));
        }

        TaskPopupOffset.BeginAnimation(System.Windows.Media.TranslateTransform.YProperty,
            new DoubleAnimation(28, 0, TimeSpan.FromMilliseconds(260)) { EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut } });
    }

    private void HideTaskPopup()
    {
        var fade = new DoubleAnimation(TaskPopup.Opacity, 0, TimeSpan.FromMilliseconds(180));
        fade.Completed += (_, _) => TaskPopup.Visibility = Visibility.Collapsed;
        TaskPopup.BeginAnimation(OpacityProperty, fade);
        TaskPopupOffset.BeginAnimation(System.Windows.Media.TranslateTransform.YProperty,
            new DoubleAnimation(TaskPopupOffset.Y, 20, TimeSpan.FromMilliseconds(180)));
    }

    /// <summary>隐藏的演示入口（--demo-download）：用来检查右下角弹窗动画和进度显示。</summary>
    private async Task RunDemoDownloadAsync()
    {
        if (DataContext is not MainViewModel vm) return;
        var task = vm.Downloads.Begin("演示任务：安装 1.20.1");
        task.Update("获取版本清单 152/920", 152, 920, 12 * 1024 * 1024, 240 * 1024 * 1024);
        for (var i = 1; i <= 24; i++)
        {
            await Task.Delay(1200);
            task.Update($"下载资源文件 {i}/24", i, 24, i * 10L * 1024 * 1024, 240 * 1024 * 1024);
        }

        vm.Downloads.Finish(task, "演示任务完成");
    }

    private void TaskPopupClose_Click(object sender, RoutedEventArgs e)
    {
        HideTaskPopup();
        _monitor?.ClearFinished();
    }

    private void Account_Click(object sender, RoutedEventArgs e)
        => AccountPopup.IsOpen = !AccountPopup.IsOpen;

    private void AccountList_SelectionChanged(object sender, System.Windows.Controls.SelectionChangedEventArgs e)
    {
        if (e.AddedItems.Count > 0) AccountPopup.IsOpen = false;
    }

    private void AddMicrosoft_Click(object sender, RoutedEventArgs e)
    {
        if (DataContext is not MainViewModel vm) return;
        AccountPopup.IsOpen = false;
        var window = new MicrosoftLoginWindow(vm.Services.Accounts) { Owner = this };
        window.ShowDialog();
    }

    private void Window_DragOver(object sender, DragEventArgs e)
    {
        e.Effects = e.Data.GetDataPresent(DataFormats.FileDrop) ? DragDropEffects.Copy : DragDropEffects.None;
        e.Handled = true;
    }

    private void Window_Drop(object sender, DragEventArgs e)
    {
        if (DataContext is not MainViewModel vm) return;
        if (e.Data.GetData(DataFormats.FileDrop) is not string[] { Length: > 0 } files) return;

        var path = files[0];
        var extension = Path.GetExtension(path);
        if (!string.Equals(extension, ".mrpack", StringComparison.OrdinalIgnoreCase)
            && !string.Equals(extension, ".zip", StringComparison.OrdinalIgnoreCase))
            return;

        e.Handled = true;
        new ModpackDropWindow(vm.Services, path) { Owner = this }.ShowDialog();
    }
}
