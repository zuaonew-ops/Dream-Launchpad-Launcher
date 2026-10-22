using System.Linq;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Markup.Xaml;
using Avalonia.Platform.Storage;
using DLP.Avalonia.ViewModels;

namespace DLP.Avalonia.Views;

/// <summary>下载页（Avalonia 版）：支持把 .mrpack / CurseForge 整合包 zip 拖进窗口安装。</summary>
public partial class DownloadPage : UserControl
{
    private static readonly string[] PackExtensions = [".mrpack", ".zip"];

    public DownloadPage()
    {
        InitializeComponent();
        DragDrop.SetAllowDrop(this, true);
        AddHandler(DragDrop.DragOverEvent, OnDragOver);
        AddHandler(DragDrop.DropEvent, OnDrop);
    }

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);

    private static void OnDragOver(object? sender, DragEventArgs e)
    {
        e.DragEffects = FindPackPath(e) is null ? DragDropEffects.None : DragDropEffects.Copy;
        e.Handled = true;
    }

    private void OnDrop(object? sender, DragEventArgs e)
    {
        if (FindPackPath(e) is not { } path) return;
        if (DataContext is DownloadViewModel viewModel) viewModel.StartLocalImport(path);
        e.Handled = true;
    }

    /// <summary>拖进来的文件里挑第一个像整合包的（.mrpack / .zip，且确实存在）。</summary>
    private static string? FindPackPath(DragEventArgs e)
    {
        // Avalonia 11.3 起用 IDataTransfer（e.Data 已标记过时）。
        var files = e.DataTransfer.TryGetFiles();
        if (files is null) return null;
        foreach (var item in files)
        {
            var path = item.TryGetLocalPath();
            if (string.IsNullOrEmpty(path)) continue;
            var extension = System.IO.Path.GetExtension(path);
            if (PackExtensions.Any(ext => string.Equals(ext, extension, StringComparison.OrdinalIgnoreCase)))
                return path;
        }
        return null;
    }
}
