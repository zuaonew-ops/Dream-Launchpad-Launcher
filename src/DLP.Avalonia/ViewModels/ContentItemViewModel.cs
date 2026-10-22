using Avalonia.Media.Imaging;
using Avalonia.Threading;
using DLP.App.Mvvm;
using DLP.Core.Download;

namespace DLP.Avalonia.ViewModels;

/// <summary>下载列表里的一行：项目信息 + 异步加载的图标（Avalonia 版用 Bitmap）。</summary>
public sealed class ContentItemViewModel(ContentProject project) : ObservableObject
{
    private Bitmap? _icon;
    private bool _iconRequested;

    public ContentProject Project { get; } = project;

    public string Id => Project.Id;
    public string Slug => Project.Slug;
    public string Title => Project.Title;
    public string Description => Project.Description;
    public string Author => Project.Author;
    public long Downloads => Project.Downloads;
    public string DownloadsText => Project.Downloads >= 1_000_000
        ? $"{Project.Downloads / 1_000_000.0:F1}M"
        : Project.Downloads >= 1000
            ? $"{Project.Downloads / 1000.0:F0}K"
            : Project.Downloads.ToString();

    public Bitmap? Icon
    {
        get => _icon;
        private set
        {
            if (!SetProperty(ref _icon, value)) return;
            OnPropertyChanged(nameof(HasIcon));
        }
    }

    public bool HasIcon => _icon is not null;

    public void LoadIcon(Services.ImageCache cache)
    {
        if (_iconRequested) return;
        _iconRequested = true;
        _ = LoadIconAsync(cache);
    }

    private async Task LoadIconAsync(Services.ImageCache cache)
    {
        var image = await cache.GetAsync(Project.IconUrl);
        if (image is null) return;
        if (Dispatcher.UIThread.CheckAccess()) Icon = image;
        else Dispatcher.UIThread.Post(() => Icon = image);
    }
}
