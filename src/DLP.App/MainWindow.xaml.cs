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
}
