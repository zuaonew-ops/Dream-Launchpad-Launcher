using System.Collections.Specialized;
using System.Windows.Controls;
using DLP.App.ViewModels;

namespace DLP.App.Views;

public partial class LogPage : UserControl
{
    public LogPage()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is LogViewModel vm)
                ((INotifyCollectionChanged)vm.Lines).CollectionChanged += (_, _) =>
                {
                    Dispatcher.BeginInvoke(new Action(() =>
                    {
                        try
                        {
                            if (vm.Lines.Count > 0) LogList.ScrollIntoView(vm.Lines[^1]);
                        }
                        catch { /* 布局未就绪时忽略 */ }
                    }), System.Windows.Threading.DispatcherPriority.Background);
                };
        };
    }
}