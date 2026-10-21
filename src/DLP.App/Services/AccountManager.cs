using System.Collections.ObjectModel;
using System.IO;
using System.Net.Http;
using System.Windows;
using DLP.App.Mvvm;
using DLP.Core.Auth;
using DLP.Core.Infrastructure;

namespace DLP.App.Services;

public sealed record AccountEntry(string Kind, string Username, string Uuid)
{
    public string KindLabel => Kind == "msa" ? "微软账号" : "离线账号";
}

/// <summary>账号管理：维护本地账号列表、当前账号、微软登录与令牌刷新。</summary>
public sealed class AccountManager
{
    private readonly GamePaths _paths;
    private readonly AppSettings _settings;
    private readonly HttpClient _http;
    private readonly AccountStore _store;
    private AccountEntry? _current;

    public AccountManager(GamePaths paths, AppSettings settings, HttpClient http)
    {
        _paths = paths;
        _settings = settings;
        _http = http;
        _store = new AccountStore(Path.Combine(paths.Root, "accounts.json"));
        Accounts = [];
        Load();
    }

    public ObservableCollection<AccountEntry> Accounts { get; }

    /// <summary>是否在设置里填了自定义客户端 ID（不填也能用内置公共客户端登录）。</summary>
    public bool HasCustomMicrosoftClientId => !string.IsNullOrWhiteSpace(_settings.MicrosoftClientId);
    public event Action? Changed;

    public AccountEntry? Current
    {
        get => _current;
        private set
        {
            _current = value;
            if (value is not null)
            {
                _settings.SelectedAccount = value.Uuid;
                // 离线用户名只在离线账号上覆盖，切到微软账号时保留原离线名。
                if (value.Kind == "offline") _settings.Username = value.Username;
                _settings.Save();
            }
            Changed?.Invoke();
        }
    }

    public string CurrentName => Current?.Username ?? "未登录";
    public string CurrentKind => Current?.KindLabel ?? "点击切换账号";
    public string CurrentInitial => string.IsNullOrEmpty(CurrentName) ? "?" : CurrentName[..1].ToUpperInvariant();

    private void Load()
    {
        var stored = _store.Load();
        foreach (var item in stored)
            Accounts.Add(new AccountEntry(item.Kind, item.Username, item.Uuid));

        if (Accounts.Count == 0)
        {
            var offline = LaunchAccount.Offline(string.IsNullOrWhiteSpace(_settings.Username) ? "Steve" : _settings.Username);
            var entry = new AccountEntry("offline", offline.Username, offline.Uuid);
            Accounts.Add(entry);
            _store.Save([new StoredAccount("offline", entry.Username, entry.Uuid, null, null, null)]);
        }

        _current = Accounts.FirstOrDefault(a => a.Uuid == _settings.SelectedAccount) ?? Accounts[0];
    }

    public void Switch(AccountEntry entry)
    {
        if (!Accounts.Contains(entry)) return;
        Current = entry;
    }

    public void SetOffline(string username)
    {
        username = string.IsNullOrWhiteSpace(username) ? "Steve" : username.Trim();
        var existing = Accounts.FirstOrDefault(a => a.Kind == "offline");
        if (existing is not null)
        {
            var index = Accounts.IndexOf(existing);
            var updated = existing with { Username = username };
            Accounts[index] = updated;
            Current = updated;
        }
        else
        {
            var offline = LaunchAccount.Offline(username);
            var entry = new AccountEntry("offline", offline.Username, offline.Uuid);
            Accounts.Add(entry);
            Current = entry;
        }
        Persist();
    }

    /// <summary>微软设备码登录；deviceCode 回调把设备码交给界面显示。</summary>
    public async Task<MicrosoftSession> LoginMicrosoftAsync(IProgress<MicrosoftDeviceCode> deviceCode, CancellationToken ct)
    {
        // 设置里没填就用内置公共客户端，用户不需要自己注册 Azure 应用。
        var auth = new MicrosoftAuthService(_http, _settings.MicrosoftClientId);
        var code = await auth.StartDeviceCodeAsync(ct);
        deviceCode.Report(code);
        var session = await auth.WaitForLoginAsync(code, null, ct);

        var entry = new AccountEntry("msa", session.Account.Username, session.Account.Uuid);
        var old = Accounts.FirstOrDefault(a => a.Uuid == entry.Uuid);
        if (old is not null) Accounts[Accounts.IndexOf(old)] = entry;
        else Accounts.Add(entry);
        Current = entry;
        PersistSession(session);
        Persist();
        return session;
    }

    public LaunchAccount GetLaunchAccount()
    {
        var current = Current ?? throw new InvalidOperationException("没有可用账号");
        if (current.Kind != "msa") return LaunchAccount.Offline(current.Username);

        var stored = _store.Load().FirstOrDefault(a => a.Uuid == current.Uuid)
            ?? throw new InvalidOperationException("微软账号信息缺失，请重新登录");
        if (stored.ExpiresAt is { } expires && expires > DateTimeOffset.UtcNow.AddMinutes(2) && !string.IsNullOrWhiteSpace(stored.AccessToken))
            return new LaunchAccount(stored.Username, stored.Uuid, stored.AccessToken, "msa");

        if (string.IsNullOrWhiteSpace(stored.RefreshToken))
            throw new InvalidOperationException("微软登录已过期，请重新登录");

        var clientId = stored.ClientId ?? _settings.MicrosoftClientId;
        var session = new MicrosoftAuthService(_http, clientId)
            .RefreshAsync(stored.RefreshToken, CancellationToken.None)
            .GetAwaiter().GetResult();
        PersistSession(session);
        return session.Account;
    }

    private void PersistSession(MicrosoftSession session)
    {
        var accounts = _store.Load().ToList();
        var index = accounts.FindIndex(a => a.Uuid == session.Account.Uuid);
        var record = new StoredAccount("msa", session.Account.Username, session.Account.Uuid,
            session.RefreshToken, session.Account.AccessToken, session.ExpiresAt,
            session.ClientId ?? _settings.MicrosoftClientId);
        if (index >= 0) accounts[index] = record; else accounts.Add(record);
        _store.Save(accounts);
    }

    private void Persist()
    {
        var records = new List<StoredAccount>();
        foreach (var entry in Accounts)
        {
            if (entry.Kind == "offline")
            {
                records.Add(new StoredAccount("offline", entry.Username, entry.Uuid, null, null, null));
                continue;
            }
            // 微软账号只保留已保存过令牌的记录，避免把 refreshToken/accessToken 覆盖成 null。
            var existing = _store.Load().FirstOrDefault(a => a.Uuid == entry.Uuid);
            if (existing is not null) records.Add(existing);
        }
        _store.Save(records);
    }
}
