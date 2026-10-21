using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;
using System.Text.Json;

namespace DLP.Core.Auth;

/// <summary>落盘的账号条目。RefreshToken / AccessToken 在文件中以 AesGcm 密文保存。</summary>
public sealed record StoredAccount(
    string Kind,
    string Username,
    string Uuid,
    string? RefreshToken,
    string? AccessToken,
    DateTimeOffset? ExpiresAt,
    string? ClientId = null);

/// <summary>
/// 账号存储：单个 JSON 文件 + 同目录的 accounts.key（32 字节随机密钥，仅当前用户可读）。
/// 令牌字段用 AES-GCM 加密，密钥首次使用时自动生成；no 明文落盘。
/// </summary>
public sealed class AccountStore
{
    /// <summary>同目录密钥文件名。</summary>
    public const string KeyFileName = "accounts.key";

    private const string CipherPrefix = "v1:";
    private const int KeySize = 32;
    private const int NonceSize = 12;
    private const int TagSize = 16;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
    };

    private readonly object _gate = new();
    private readonly string _filePath;
    private byte[]? _key;

    public AccountStore(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath))
            throw new ArgumentException("账号文件路径不能为空", nameof(filePath));
        _filePath = Path.GetFullPath(filePath);
    }

    /// <summary>账号文件绝对路径。</summary>
    public string FilePath => _filePath;

    /// <summary>密钥文件绝对路径（与账号文件同目录）。</summary>
    public string KeyPath => Path.Combine(Path.GetDirectoryName(_filePath) ?? throw new InvalidOperationException("账号文件路径无效"), KeyFileName);

    /// <summary>读取全部账号；文件不存在时返回空列表。</summary>
    public IReadOnlyList<StoredAccount> Load()
    {
        lock (_gate)
        {
            if (!File.Exists(_filePath)) return Array.Empty<StoredAccount>();

            var text = File.ReadAllText(_filePath, Encoding.UTF8);
            if (string.IsNullOrWhiteSpace(text)) return Array.Empty<StoredAccount>();

            List<AccountFileEntry> entries;
            try
            {
                entries = text.TrimStart().StartsWith('[')
                    ? JsonSerializer.Deserialize<List<AccountFileEntry>>(text, JsonOptions) ?? new List<AccountFileEntry>()
                    : JsonSerializer.Deserialize<AccountFile>(text, JsonOptions)?.Accounts ?? new List<AccountFileEntry>();
            }
            catch (JsonException ex)
            {
                throw new InvalidOperationException($"账号文件格式不正确：{_filePath}（{ex.Message}）", ex);
            }

            var result = new List<StoredAccount>(entries.Count);
            foreach (var entry in entries)
            {
                if (entry is null || string.IsNullOrWhiteSpace(entry.Username)) continue;

                result.Add(new StoredAccount(
                    string.IsNullOrWhiteSpace(entry.Kind) ? "msa" : entry.Kind,
                    entry.Username,
                    entry.Uuid ?? string.Empty,
                    DecryptToken(entry.RefreshToken, entry.Username),
                    DecryptToken(entry.AccessToken, entry.Username),
                    entry.ExpiresAt));
            }

            return result;
        }
    }

    /// <summary>覆盖写入全部账号（先写临时文件再替换，避免半截文件）。</summary>
    public void Save(IEnumerable<StoredAccount> accounts)
    {
        ArgumentNullException.ThrowIfNull(accounts);

        lock (_gate)
        {
            var directory = Path.GetDirectoryName(_filePath);
            if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);

            var entries = accounts.Select(account => new AccountFileEntry
            {
                Kind = account.Kind,
                Username = account.Username,
                Uuid = account.Uuid,
                RefreshToken = EncryptToken(account.RefreshToken),
                AccessToken = EncryptToken(account.AccessToken),
                ExpiresAt = account.ExpiresAt,
            }).ToList();

            var json = JsonSerializer.Serialize(new AccountFile { Version = 1, Accounts = entries }, JsonOptions);

            var temp = _filePath + ".tmp";
            File.WriteAllText(temp, json, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
            File.Move(temp, _filePath, overwrite: true);
        }
    }

    private string? EncryptToken(string? plaintext)
    {
        if (string.IsNullOrEmpty(plaintext)) return null;

        var key = GetOrCreateKey();
        var nonce = RandomNumberGenerator.GetBytes(NonceSize);
        var plainBytes = Encoding.UTF8.GetBytes(plaintext);
        var cipher = new byte[plainBytes.Length];
        var tag = new byte[TagSize];

        using (var aes = new AesGcm(key, TagSize))
            aes.Encrypt(nonce, plainBytes, cipher, tag);

        var payload = new byte[NonceSize + cipher.Length + TagSize];
        Buffer.BlockCopy(nonce, 0, payload, 0, NonceSize);
        Buffer.BlockCopy(cipher, 0, payload, NonceSize, cipher.Length);
        Buffer.BlockCopy(tag, 0, payload, NonceSize + cipher.Length, TagSize);

        return CipherPrefix + Convert.ToBase64String(payload);
    }

    private string? DecryptToken(string? stored, string username)
    {
        if (string.IsNullOrEmpty(stored)) return null;
        if (!stored.StartsWith(CipherPrefix, StringComparison.Ordinal)) return stored; // 兼容手工写入的明文

        byte[] payload;
        try { payload = Convert.FromBase64String(stored[CipherPrefix.Length..]); }
        catch (FormatException ex)
        {
            throw new InvalidOperationException($"账号 {username} 的凭据不是合法的 Base64，无法解密：{_filePath}", ex);
        }

        if (payload.Length <= NonceSize + TagSize)
            throw new InvalidOperationException($"账号 {username} 的凭据已损坏（长度 {payload.Length}）：{_filePath}");

        var key = GetOrCreateKey();
        var nonce = payload.AsSpan(0, NonceSize);
        var cipher = payload.AsSpan(NonceSize, payload.Length - NonceSize - TagSize);
        var tag = payload.AsSpan(payload.Length - TagSize, TagSize);
        var plain = new byte[cipher.Length];

        try
        {
            using var aes = new AesGcm(key, TagSize);
            aes.Decrypt(nonce, cipher, tag, plain);
        }
        catch (CryptographicException ex)
        {
            throw new InvalidOperationException(
                $"账号 {username} 的凭据解密失败：密钥文件 {KeyPath} 可能已更换或被删除，请重新登录该账号。", ex);
        }

        return Encoding.UTF8.GetString(plain);
    }

    private byte[] GetOrCreateKey()
    {
        if (_key is not null) return _key;

        var keyPath = KeyPath;
        if (File.Exists(keyPath))
        {
            var existing = File.ReadAllBytes(keyPath);
            if (existing.Length == KeySize) return _key = existing;

            throw new InvalidOperationException(
                $"账号密钥文件已损坏：{keyPath}（长度 {existing.Length} 字节，应为 {KeySize} 字节）。删除该文件后重新登录即可重建。");
        }

        var directory = Path.GetDirectoryName(keyPath);
        if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);

        var key = RandomNumberGenerator.GetBytes(KeySize);
        // 先建空文件并收紧权限，再写入内容：WriteAllBytes 会保留已有 ACL。
        using (new FileStream(keyPath, FileMode.CreateNew, FileAccess.Write, FileShare.None)) { }
        RestrictToCurrentUser(keyPath);
        File.WriteAllBytes(keyPath, key);

        return _key = key;
    }

    /// <summary>尽量把文件权限收成“仅当前用户读写”；无权限调整时静默退化为目录默认权限。</summary>
    private static void RestrictToCurrentUser(string path)
    {
        try
        {
            if (OperatingSystem.IsWindows())
            {
                var sid = WindowsIdentity.GetCurrent().User;
                if (sid is null) return;

                var security = new FileSecurity();
                security.SetAccessRuleProtection(isProtected: true, preserveInheritance: false);
                security.AddAccessRule(new FileSystemAccessRule(sid, FileSystemRights.FullControl, AccessControlType.Allow));
                new FileInfo(path).SetAccessControl(security);
            }
            else
            {
                File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
            }
        }
        catch (Exception ex) when (ex is UnauthorizedAccessException or IOException or PlatformNotSupportedException
                                       or InvalidOperationException or PrivilegeNotHeldException)
        {
            // 权限收紧是尽力而为，失败不应阻断登录。
        }
    }

    private sealed class AccountFile
    {
        public int Version { get; set; } = 1;
        public List<AccountFileEntry> Accounts { get; set; } = new();
    }

    private sealed class AccountFileEntry
    {
        public string? Kind { get; set; }
        public string Username { get; set; } = string.Empty;
        public string? Uuid { get; set; }
        public string? RefreshToken { get; set; }
        public string? AccessToken { get; set; }
        public DateTimeOffset? ExpiresAt { get; set; }
    }
}
