using System.IO;
using System.Windows;
using DLP.App.ViewModels;

namespace DLP.App;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
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
