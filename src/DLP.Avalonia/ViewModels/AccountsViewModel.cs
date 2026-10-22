using System.Collections.ObjectModel;
using Avalonia.Threading;
using DLP.App.Mvvm;
using DLP.App.Services;
using DLP.Avalonia.Services;

namespace DLP.Avalonia.ViewModels;

/// <summary>账号管理（Avalonia）：添加离线账号、切换当前账号、删除账号。</summary>
public sealed class AccountsViewModel : ObservableObject
{
    private readonly AppServices _services;
    private AccountEntry? _selected;
    private string _newOfflineName = "";
    private AccountEntry? _pendingRemove;
    private string _status;

    public AccountsViewModel(AppServices services)
    {
        _services = services;
        Accounts = services.Accounts.Accounts;
        _selected = services.Accounts.Current;
        _status = $"当前账号：{services.Accounts.CurrentName}";

        AddOfflineCommand = new RelayCommand(AddOffline);
        SwitchCommand = new RelayCommand(Switch, () => Selected is not null && !ReferenceEquals(Selected, _services.Accounts.Current));
        RequestRemoveCommand = new RelayCommand(RequestRemove, () => Selected is not null);
        ConfirmRemoveCommand = new RelayCommand(ConfirmRemove);
        CancelRemoveCommand = new RelayCommand(CancelRemove);

        _services.Accounts.Changed += OnAccountsChanged;
    }

    public ObservableCollection<AccountEntry> Accounts { get; }
    public RelayCommand AddOfflineCommand { get; }
    public RelayCommand SwitchCommand { get; }
    public RelayCommand RequestRemoveCommand { get; }
    public RelayCommand ConfirmRemoveCommand { get; }
    public RelayCommand CancelRemoveCommand { get; }

    public AccountEntry? Selected
    {
        get => _selected;
        set
        {
            if (!SetProperty(ref _selected, value)) return;
            SwitchCommand.RaiseCanExecuteChanged();
            RequestRemoveCommand.RaiseCanExecuteChanged();
            if (_pendingRemove is not null && !ReferenceEquals(_pendingRemove, value)) CancelRemove();
        }
    }

    public AccountEntry? CurrentAccount => _services.Accounts.Current;
    public string CurrentName => _services.Accounts.CurrentName;
    public string CurrentKind => _services.Accounts.CurrentKind;
    public int AccountCount => Accounts.Count;

    public string NewOfflineName
    {
        get => _newOfflineName;
        set => SetProperty(ref _newOfflineName, value);
    }

    public string Status
    {
        get => _status;
        set => SetProperty(ref _status, value);
    }

    public bool IsConfirmingRemove => _pendingRemove is not null;
    public string PendingRemoveName => _pendingRemove?.Username ?? "";
    public string ConfirmMessage => _pendingRemove is { } entry
        ? $"确定删除账号「{entry.Username}」？删除后需要重新添加。"
        : "";

    private void OnAccountsChanged()
    {
        if (!Dispatcher.UIThread.CheckAccess())
        {
            Dispatcher.UIThread.Post(OnAccountsChanged);
            return;
        }

        if (_selected is null || !Accounts.Contains(_selected)) _selected = _services.Accounts.Current;
        OnPropertyChanged(nameof(Selected));
        OnPropertyChanged(nameof(CurrentAccount));
        OnPropertyChanged(nameof(CurrentName));
        OnPropertyChanged(nameof(CurrentKind));
        OnPropertyChanged(nameof(AccountCount));
        SwitchCommand.RaiseCanExecuteChanged();
        RequestRemoveCommand.RaiseCanExecuteChanged();
    }

    private void AddOffline()
    {
        var name = string.IsNullOrWhiteSpace(NewOfflineName) ? "Steve" : NewOfflineName.Trim();
        var entry = _services.Accounts.AddOffline(name);
        NewOfflineName = "";
        Selected = entry;
        Status = $"已添加离线账号「{entry.Username}」并切换到该账号";
        _services.Log.Info($"添加离线账号 {entry.Username}");
    }

    private void Switch()
    {
        if (Selected is not { } entry) return;
        _services.Accounts.Switch(entry);
        Status = $"已切换到「{entry.Username}」";
        _services.Log.Info($"切换账号 {entry.Username}");
    }

    private void RequestRemove()
    {
        if (Selected is not { } entry) return;
        if (Accounts.Count <= 1)
        {
            Status = "至少要保留一个账号";
            return;
        }

        _pendingRemove = entry;
        NotifyRemoveState();
        Status = $"待确认删除账号：{entry.Username}";
    }

    private void CancelRemove()
    {
        if (_pendingRemove is null) return;
        _pendingRemove = null;
        NotifyRemoveState();
        Status = "已取消删除";
    }

    private void ConfirmRemove()
    {
        if (_pendingRemove is not { } entry) return;
        if (Accounts.Count <= 1)
        {
            Status = "至少要保留一个账号";
            _pendingRemove = null;
            NotifyRemoveState();
            return;
        }

        var name = entry.Username;
        var removed = _services.Accounts.RemoveAccount(entry);
        if (Accounts.Count == 0) _services.Accounts.AddOffline("Steve");

        _pendingRemove = null;
        NotifyRemoveState();
        Selected = _services.Accounts.Current;
        Status = removed ? $"已删除账号「{name}」" : $"账号「{name}」不存在，可能已被删除";
        _services.Log.Info($"删除账号 {name}");
    }

    private void NotifyRemoveState()
    {
        OnPropertyChanged(nameof(IsConfirmingRemove));
        OnPropertyChanged(nameof(PendingRemoveName));
        OnPropertyChanged(nameof(ConfirmMessage));
    }
}
