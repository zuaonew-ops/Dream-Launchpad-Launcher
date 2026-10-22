using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace DLP.Avalonia.Views;

public partial class HomeView : UserControl
{
    public HomeView() => InitializeComponent();

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);
}
