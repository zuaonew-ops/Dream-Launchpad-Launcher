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

    /// <summary>把登录失败原因写到 logs/auth.log，方便排查。</summary>
    public void LogAuthFailure(string detail)
    {
        try
        {
            var dir = Path.Combine(_paths.Root, "logs");
            Directory.CreateDirectory(dir);
            File.AppendAllText(Path.Combine(dir, "auth.log"),
                $"[{DateTimeOffset.Now:yyyy-MM-dd HH:mm:ss}] {detail}{Environment.NewLine}");
        }
        catch (IOException)
        {
        }
    }

    /// <summary>把客户端 ID 写进设置（登录窗口里直接填的那种）。</summary>
    public void SaveMicrosoftClientId(string clientId)
    {
        _settings.MicrosoftClientId = string.IsNullOrWhiteSpace(clientId) ? null : clientId.Trim();
        _settings.Save();
    }

    /// <summary>当前解析出的微软客户端 ID：设置页 &gt; 数据目录 client_id.txt &gt; 编译期默认值。</summary>
    public string? ResolveMicrosoftClientId()
    {
        var fromSettings = _settings.MicrosoftClientId?.Trim();
        if (!string.IsNullOrEmpty(fromSettings)) return fromSettings;

        try
        {
            var file = Path.Combine(_paths.Root, "client_id.txt");
            if (File.Exists(file))
            {
                var text = File.ReadAllText(file).Trim();
                if (text.Length > 0) return text;
            }
        }
        catch (IOException)
        {
            // 读不到文件就继续看默认值。
        }

        var fallback = MicrosoftAuthService.DefaultClientId.Trim();
        return fallback.Length > 0 ? fallback : null;
    }
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
        var clientId = ResolveMicrosoftClientId()
            ?? throw new InvalidOperationException("未配置微软客户端 ID：到设置页填入 Azure 公共客户端 ID，或把 ID 写进数据目录的 client_id.txt");
        var auth = new MicrosoftAuthService(_http, clientId);
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

    /// <summary>网页登录完成后：用 Microsoft access_token 换 Minecraft 账号并保存。</summary>
    public async Task<MicrosoftSession> CompleteWebLoginAsync(string microsoftAccessToken, string? refreshToken, CancellationToken ct)
    {
        var auth = new MicrosoftAuthService(_http, null);
        var session = await auth.LoginWithMicrosoftTokenAsync(microsoftAccessToken, refreshToken, ct);

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

        var clientId = stored.ClientId ?? ResolveMicrosoftClientId()
            ?? throw new InvalidOperationException("未配置微软客户端 ID，无法刷新登录");
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
            session.ClientId ?? ResolveMicrosoftClientId());
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
