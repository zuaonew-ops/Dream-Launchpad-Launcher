using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace DLP.Avalonia.Views;

public partial class AccountsView : UserControl
{
    public AccountsView() => InitializeComponent();

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);
}
