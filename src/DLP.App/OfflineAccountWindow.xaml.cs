using System.Windows;

namespace DLP.App;

public partial class OfflineAccountWindow : Window
{
    public OfflineAccountWindow(string? initial = null)
    {
        InitializeComponent();
        NameBox.Text = initial ?? "";
        Loaded += (_, _) => { NameBox.Focus(); NameBox.SelectAll(); };
    }

    public string Username => NameBox.Text.Trim();

    private void Ok_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrWhiteSpace(NameBox.Text))
        {
            MessageBox.Show("请输入离线用户名", "添加离线账号", MessageBoxButton.OK, MessageBoxImage.Information);
            return;
        }

        DialogResult = true;
    }
}
