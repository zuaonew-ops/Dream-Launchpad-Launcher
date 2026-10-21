using System.Windows.Media;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Core.Download;

namespace DLP.App.ViewModels;

/// <summary>下载列表里的一行：项目信息 + 异步加载的图标。</summary>
public sealed class ContentItemViewModel(ContentProject project) : ObservableObject
{
    private ImageSource? _icon;
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

    public ImageSource? Icon
    {
        get => _icon;
        private set
        {
            if (!SetProperty(ref _icon, value)) return;
            OnPropertyChanged(nameof(HasIcon));
        }
    }

    public bool HasIcon => _icon is not null;

    public void LoadIcon(ImageCache cache)
    {
        if (_iconRequested) return;
        _iconRequested = true;
        _ = LoadIconAsync(cache);
    }

    private async Task LoadIconAsync(ImageCache cache)
    {
        var image = await cache.LoadAsync(Project.IconUrl);
        if (image is null) return;
        await System.Windows.Application.Current.Dispatcher.InvokeAsync(() => Icon = image);
    }
}
