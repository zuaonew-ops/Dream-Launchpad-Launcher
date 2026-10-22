using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using DLP.Avalonia.ViewModels;

namespace DLP.Avalonia.Views;

public partial class MainView : UserControl
{
    public MainView()
    {
        InitializeComponent();
        SizeChanged += (_, _) => UpdateLayoutMode();
    }

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);

    /// <summary>窗口/屏幕比 720px 窄时切到顶部标签栏布局。</summary>
    private void UpdateLayoutMode()
    {
        if (DataContext is MainViewModel vm)
            vm.IsCompact = Bounds.Width > 0 && Bounds.Width < 720;
    }
}
