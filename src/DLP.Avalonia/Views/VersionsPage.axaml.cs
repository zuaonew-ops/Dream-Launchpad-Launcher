using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace DLP.Avalonia.Views;

public partial class VersionsPage : UserControl
{
    public VersionsPage() => InitializeComponent();

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);
}
