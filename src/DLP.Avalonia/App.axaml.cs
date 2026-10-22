using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using DLP.App.Services;
using DLP.Avalonia.Services;
using DLP.Avalonia.ViewModels;
using DLP.Avalonia.Views;

namespace DLP.Avalonia;

public partial class App : Application
{
    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
    {
        // 共享服务（DLP.Common）通过这个回调回到界面线程。
        UiDispatcher.Post = action => global::Avalonia.Threading.Dispatcher.UIThread.Post(action);

        var services = new AppServices();
        var viewModel = MainViewModel.Create(services);

        switch (ApplicationLifetime)
        {
            case IClassicDesktopStyleApplicationLifetime desktop:
                desktop.MainWindow = new MainWindow { DataContext = viewModel };
                break;
            case ISingleViewApplicationLifetime singleView:
                singleView.MainView = new MainView { DataContext = viewModel };
                break;
        }

        ThemeManager.Apply(services.Settings.Theme);
        base.OnFrameworkInitializationCompleted();
    }
}
