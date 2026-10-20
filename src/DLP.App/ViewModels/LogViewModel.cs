using System.Collections.ObjectModel;
using System.Windows;
using DLP.App.Mvvm;
using DLP.App.Services;

namespace DLP.App.ViewModels;

public sealed class LogViewModel : ObservableObject
{
    private const int MaxLines = 2000;

    public LogViewModel(AppServices services)
    {
        Lines = [];
        ClearCommand = new RelayCommand(Clear);
        services.Log.Written += (_, line) => Application.Current.Dispatcher.BeginInvoke(() =>
        {
            Lines.Add(line);
            if (Lines.Count > MaxLines) Lines.RemoveAt(0);
        });
    }

    public ObservableCollection<string> Lines { get; }
    public RelayCommand ClearCommand { get; }

    private void Clear() => Lines.Clear();
}
