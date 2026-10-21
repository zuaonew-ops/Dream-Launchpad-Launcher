using System.Net.Http.Headers;
using System.Text.Json;

namespace DLP.Core.Auth;

/// <summary>OAuth 2.0 设备码流程返回给用户的登录信息。</summary>
public sealed record MicrosoftDeviceCode(
    string DeviceCode,
    string UserCode,
    string VerificationUri,
    int IntervalSeconds,
    DateTimeOffset ExpiresAt,
    string Message);

/// <summary>一次微软登录（或刷新）得到的会话。</summary>
/// <param name="ExpiresAt">Minecraft 访问令牌的到期时间（UTC）。</param>
public sealed record MicrosoftSession(LaunchAccount Account, string RefreshToken, DateTimeOffset ExpiresAt, string? ClientId = null);

/// <summary>
/// 微软账号登录：设备码流程拿 Microsoft 令牌，再依次换 Xbox Live、XSTS、Minecraft Services 令牌，
/// 最后读取 Minecraft 档案得到玩家名与 UUID。
/// </summary>
public sealed class MicrosoftAuthService
{
    private const string DeviceCodeEndpoint = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
    private const string TokenEndpoint = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    private const string Scope = "XboxLive.signin offline_access";
    private const string XboxLiveEndpoint = "https://user.auth.xboxlive.com/user/authenticate";
    private const string XstsEndpoint = "https://xsts.auth.xboxlive.com/xsts/authorize";
    private const string MinecraftLoginEndpoint = "https://api.minecraftservices.com/authentication/login_with_xbox";
    private const string MinecraftProfileEndpoint = "https://api.minecraftservices.com/minecraft/profile";
    private const string DefaultVerificationUri = "https://www.microsoft.com/link";

    /// <summary>
    /// 默认客户端 ID：可以通过 MSBuild 属性注入（发布时 <c>-p:MicrosoftClientId=xxx</c>），
    /// 或者放在数据目录的 client_id.txt / 设置页里。留空时登录会给出注册引导。
    /// </summary>
    public static string DefaultClientId { get; set; } = "9c8a21d5-5f66-4dba-b90a-42e47d2aaac1";

    private readonly HttpClient _http;
    private string _clientId;

    public MicrosoftAuthService(HttpClient http, string? clientId)
    {
        _http = http ?? throw new ArgumentNullException(nameof(http));
        _clientId = string.IsNullOrWhiteSpace(clientId) ? DefaultClientId.Trim() : clientId.Trim();
    }

    /// <summary>当前实际使用的客户端 ID；登录成功后随账号一起保存，续期时沿用。</summary>
    public string ClientId => _clientId;

    /// <summary>申请设备码，用户需在浏览器打开 <see cref="MicrosoftDeviceCode.VerificationUri"/> 并输入 UserCode。</summary>
    public async Task<MicrosoftDeviceCode> StartDeviceCodeAsync(CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(_clientId))
            throw new InvalidOperationException("未配置微软客户端 ID：需要先在 Azure 门户注册一个公共客户端（见设置页说明）");

        var response = await PostFormAsync(DeviceCodeEndpoint, new Dictionary<string, string>
        {
            ["client_id"] = _clientId,
            ["scope"] = Scope,
        }, ct).ConfigureAwait(false);

        ThrowIfFailed(response, "申请微软登录设备码失败");
        using var doc = response.Json ?? throw new InvalidOperationException("申请微软登录设备码失败：服务器返回空响应");

        var root = doc.RootElement;
        var deviceCode = GetString(root, "device_code")
            ?? throw new InvalidOperationException("申请微软登录设备码失败：响应缺少 device_code");
        var userCode = GetString(root, "user_code")
            ?? throw new InvalidOperationException("申请微软登录设备码失败：响应缺少 user_code");
        var verificationUri = GetString(root, "verification_uri")
            ?? GetString(root, "verification_uri_complete")
            ?? DefaultVerificationUri;

        var interval = Math.Max(1, GetInt(root, "interval") ?? 5);
        var expiresIn = Math.Max(1, GetInt(root, "expires_in") ?? 900);
        var expiresAt = DateTimeOffset.UtcNow.AddSeconds(expiresIn);
        var message = GetString(root, "message")
            ?? $"请打开 {verificationUri} 并输入代码 {userCode}，{expiresIn / 60} 分钟内有效。";

        return new MicrosoftDeviceCode(deviceCode, userCode, verificationUri, interval, expiresAt, message);
    }

    /// <summary>轮询设备码直到用户完成授权，然后换取 Minecraft 账号。</summary>
    public async Task<MicrosoftSession> WaitForLoginAsync(MicrosoftDeviceCode code, IProgress<string>? progress, CancellationToken ct)
    {
        ArgumentNullException.ThrowIfNull(code);

        progress?.Report($"请在浏览器打开 {code.VerificationUri}，输入代码 {code.UserCode}");

        var token = await PollForTokenAsync(code, progress, ct).ConfigureAwait(false);
        var session = await ExchangeAsync(token.AccessToken, token.RefreshToken, ct).ConfigureAwait(false);
        progress?.Report($"微软登录成功：{session.Account.Username}");
        return session;
    }

    /// <summary>
    /// 网页登录（旧版 Live 隐式流）拿到 Microsoft access_token 后，继续换 XBL / XSTS / Minecraft 令牌。
    /// </summary>
    public Task<MicrosoftSession> LoginWithMicrosoftTokenAsync(string microsoftAccessToken, string? refreshToken, CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(microsoftAccessToken);
        return ExchangeAsync(microsoftAccessToken, refreshToken ?? string.Empty, ct);
    }

    /// <summary>用 refresh_token 静默续期，无需用户再次授权。</summary>
    public async Task<MicrosoftSession> RefreshAsync(string refreshToken, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(refreshToken))
            throw new ArgumentException("刷新令牌为空，无法续期微软账号", nameof(refreshToken));

        var response = await PostFormAsync(TokenEndpoint, new Dictionary<string, string>
        {
            ["grant_type"] = "refresh_token",
            ["client_id"] = _clientId,
            ["refresh_token"] = refreshToken,
            ["scope"] = Scope,
        }, ct).ConfigureAwait(false);

        using var doc = response.Json;
        var error = doc is null ? null : GetString(doc.RootElement, "error");
        if (!response.IsSuccess || error is not null)
        {
            if (error == "invalid_grant")
                throw new InvalidOperationException("微软刷新令牌已失效，请重新登录（刷新令牌可能已过期或被吊销）");
            ThrowIfFailed(response, "微软账号续期失败");
            throw new InvalidOperationException($"微软账号续期失败：{DescribeError(response.Json) ?? "未知错误"}");
        }

        var root = doc!.RootElement;
        var accessToken = GetString(root, "access_token")
            ?? throw new InvalidOperationException("微软账号续期失败：响应缺少 access_token");
        var newRefresh = GetString(root, "refresh_token");
        if (string.IsNullOrWhiteSpace(newRefresh)) newRefresh = refreshToken;

        return await ExchangeAsync(accessToken, newRefresh!, ct).ConfigureAwait(false);
    }

    private async Task<(string AccessToken, string RefreshToken)> PollForTokenAsync(
        MicrosoftDeviceCode code, IProgress<string>? progress, CancellationToken ct)
    {
        var interval = Math.Max(1, code.IntervalSeconds);

        while (true)
        {
            ct.ThrowIfCancellationRequested();

            var remaining = code.ExpiresAt - DateTimeOffset.UtcNow;
            if (remaining <= TimeSpan.Zero)
                throw new InvalidOperationException("设备码已过期，请重新发起微软登录");

            var delay = TimeSpan.FromSeconds(interval);
            if (delay > remaining) delay = remaining;
            await Task.Delay(delay, ct).ConfigureAwait(false);

            var response = await PostFormAsync(TokenEndpoint, new Dictionary<string, string>
            {
                ["grant_type"] = "urn:ietf:params:oauth:grant-type:device_code",
                ["client_id"] = _clientId,
                ["device_code"] = code.DeviceCode,
            }, ct).ConfigureAwait(false);

            using var doc = response.Json;
            var error = doc is null ? null : GetString(doc.RootElement, "error");
            if (response.IsSuccess && error is null && doc is not null)
            {
                var accessToken = GetString(doc.RootElement, "access_token")
                    ?? throw new InvalidOperationException("微软登录失败：令牌响应缺少 access_token");
                var refreshToken = GetString(doc.RootElement, "refresh_token") ?? string.Empty;
                return (accessToken, refreshToken);
            }

            switch (error)
            {
                case "authorization_pending":
                    progress?.Report("等待你在浏览器中完成微软账号授权…");
                    continue;
                case "slow_down":
                    interval += 5;
                    progress?.Report("服务器要求放慢轮询，已延长等待间隔…");
                    continue;
                case "expired_token":
                    throw new InvalidOperationException("设备码已过期，请重新发起微软登录");
                case "bad_verification_code":
                    throw new InvalidOperationException("设备码无效，请重新发起微软登录");
                case "authorization_declined":
                    throw new InvalidOperationException("你在浏览器中拒绝了微软账号登录授权");
            }

            ThrowIfFailed(response, "微软登录失败");
            throw new InvalidOperationException($"微软登录失败：未知错误（HTTP {response.StatusCode}）");
        }
    }

    /// <summary>Microsoft 令牌 -> Xbox Live -> XSTS -> Minecraft -> 档案。</summary>
    private async Task<MicrosoftSession> ExchangeAsync(string msAccessToken, string refreshToken, CancellationToken ct)
    {
        var (xblToken, _) = await XboxLiveAuthenticateAsync(msAccessToken, ct).ConfigureAwait(false);
        var (xstsToken, userHash) = await XstsAuthorizeAsync(xblToken, ct).ConfigureAwait(false);
        var (mcAccessToken, mcExpiresAt) = await MinecraftLoginAsync(userHash, xstsToken, ct).ConfigureAwait(false);
        var (username, uuid) = await GetProfileAsync(mcAccessToken, ct).ConfigureAwait(false);

        var account = new LaunchAccount(username, uuid, mcAccessToken, "msa");
        return new MicrosoftSession(account, refreshToken, mcExpiresAt, _clientId);
    }

    private async Task<(string Token, string UserHash)> XboxLiveAuthenticateAsync(string msAccessToken, CancellationToken ct)
    {
        var payload = new
        {
            Properties = new
            {
                AuthMethod = "RPS",
                SiteName = "user.auth.xboxlive.com",
                RpsTicket = "d=" + msAccessToken,
            },
            RelyingParty = "http://auth.xboxlive.com",
            TokenType = "JWT",
        };

        var response = await PostJsonAsync(XboxLiveEndpoint, payload, ct).ConfigureAwait(false);
        ThrowIfFailed(response, "Xbox Live 身份验证失败");
        using var doc = response.Json ?? throw new InvalidOperationException("Xbox Live 身份验证失败：服务器返回空响应");

        var token = GetString(doc.RootElement, "Token")
            ?? throw new InvalidOperationException("Xbox Live 身份验证失败：响应缺少 Token");
        var userHash = GetUserHash(doc.RootElement)
            ?? throw new InvalidOperationException("Xbox Live 身份验证失败：响应缺少 user hash");
        return (token, userHash);
    }

    private async Task<(string Token, string UserHash)> XstsAuthorizeAsync(string xblToken, CancellationToken ct)
    {
        var payload = new
        {
            Properties = new
            {
                SandboxId = "RETAIL",
                UserTokens = new[] { xblToken },
            },
            RelyingParty = "rp://api.minecraftservices.com/",
            TokenType = "JWT",
        };

        var response = await PostJsonAsync(XstsEndpoint, payload, ct).ConfigureAwait(false);
        if (!response.IsSuccess)
        {
            var xerr = ReadXErr(response.Json);
            throw new InvalidOperationException(xerr switch
            {
                2148916233 => "该微软账号还没有 Xbox 账号：请先打开 https://www.xbox.com 用同一账号登录一次，创建 Xbox 档案后再登录。",
                2148916235 => "该微软账号所在地区不支持 Xbox Live（账号可能注册在受限制地区），无法登录 Minecraft。",
                2148916238 => "该微软账号是未成年人账号（未满 18 岁），需要家长先将其加入 Microsoft 家庭组后才能登录。",
                _ => $"XSTS 授权失败（XErr={xerr?.ToString() ?? "未知"}，HTTP {response.StatusCode}）：{DescribeError(response.Json) ?? "Xbox Live 拒绝授权"}",
            });
        }

        using var doc = response.Json ?? throw new InvalidOperationException("XSTS 授权失败：服务器返回空响应");
        var token = GetString(doc.RootElement, "Token")
            ?? throw new InvalidOperationException("XSTS 授权失败：响应缺少 Token");
        var userHash = GetUserHash(doc.RootElement)
            ?? throw new InvalidOperationException("XSTS 授权失败：响应缺少 user hash");
        return (token, userHash);
    }

    private async Task<(string AccessToken, DateTimeOffset ExpiresAt)> MinecraftLoginAsync(
        string userHash, string xstsToken, CancellationToken ct)
    {
        var payload = new { identityToken = $"XBL3.0 x={userHash};{xstsToken}" };

        var response = await PostJsonAsync(MinecraftLoginEndpoint, payload, ct).ConfigureAwait(false);
        ThrowIfFailed(response, "Minecraft 身份验证失败");
        using var doc = response.Json ?? throw new InvalidOperationException("Minecraft 身份验证失败：服务器返回空响应");

        var accessToken = GetString(doc.RootElement, "access_token")
            ?? throw new InvalidOperationException("Minecraft 身份验证失败：响应缺少 access_token");
        var expiresIn = Math.Max(60, GetInt(doc.RootElement, "expires_in") ?? 86_400);
        return (accessToken, DateTimeOffset.UtcNow.AddSeconds(expiresIn));
    }

    private async Task<(string Username, string Uuid)> GetProfileAsync(string mcAccessToken, CancellationToken ct)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, MinecraftProfileEndpoint);
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", mcAccessToken);

        var response = await SendAsync(request, ct).ConfigureAwait(false);
        if (response.StatusCode == 404)
            throw new InvalidOperationException("该微软账号未拥有 Minecraft: Java Edition，请先在 Microsoft Store 购买游戏。");
        ThrowIfFailed(response, "读取 Minecraft 档案失败");

        using var doc = response.Json ?? throw new InvalidOperationException("读取 Minecraft 档案失败：服务器返回空响应");
        var root = doc.RootElement;
        var name = GetString(root, "name")
            ?? throw new InvalidOperationException("读取 Minecraft 档案失败：响应缺少玩家名");
        var id = GetString(root, "id")
            ?? throw new InvalidOperationException("读取 Minecraft 档案失败：响应缺少玩家 UUID");

        return (name, id.Replace("-", string.Empty).ToLowerInvariant());
    }

    private void EnsureClientId()
    {
        if (string.IsNullOrWhiteSpace(_clientId))
            throw new InvalidOperationException("未配置微软客户端 ID");
    }

    private Task<JsonResponse> PostFormAsync(string url, Dictionary<string, string> fields, CancellationToken ct)
    {
        var request = new HttpRequestMessage(HttpMethod.Post, url) { Content = new FormUrlEncodedContent(fields) };
        return SendAsync(request, ct);
    }

    private Task<JsonResponse> PostJsonAsync<T>(string url, T payload, CancellationToken ct)
    {
        var json = JsonSerializer.Serialize(payload);
        var request = new HttpRequestMessage(HttpMethod.Post, url)
        {
            Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json"),
        };
        request.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        return SendAsync(request, ct);
    }

    private async Task<JsonResponse> SendAsync(HttpRequestMessage request, CancellationToken ct)
    {
        using (request)
        using (var response = await _http.SendAsync(request, HttpCompletionOption.ResponseContentRead, ct).ConfigureAwait(false))
        {
            var text = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
            JsonDocument? doc = null;
            if (!string.IsNullOrWhiteSpace(text))
            {
                try { doc = JsonDocument.Parse(text); }
                catch (JsonException) { doc = null; }
            }

            return new JsonResponse(doc, (int)response.StatusCode, response.IsSuccessStatusCode);
        }
    }

    /// <summary>令牌只允许出现前 6 位，避免日志泄漏。</summary>
    internal static string Mask(string? token)
        => string.IsNullOrEmpty(token) ? "(空)" : token.Length <= 6 ? "******" : token[..6] + "******";

    private static void ThrowIfFailed(JsonResponse response, string action)
    {
        if (response.IsSuccess) return;
        var description = DescribeError(response.Json);
        var suffix = string.IsNullOrWhiteSpace(description) ? string.Empty : $"：{description}";
        throw new InvalidOperationException($"{action}{suffix}（HTTP {response.StatusCode}）");
    }

    private static string? DescribeError(JsonDocument? doc)
    {
        if (doc is null) return null;
        var root = doc.RootElement;
        return GetString(root, "error_description")
            ?? GetString(root, "errorMessage")
            ?? GetString(root, "error")
            ?? GetString(root, "Message")
            ?? GetString(root, "message");
    }

    private static string? GetUserHash(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object) return null;
        if (!root.TryGetProperty("DisplayClaims", out var claims) || claims.ValueKind != JsonValueKind.Object) return null;
        if (!claims.TryGetProperty("xui", out var xui) || xui.ValueKind != JsonValueKind.Array) return null;
        foreach (var entry in xui.EnumerateArray())
        {
            var uhs = GetString(entry, "uhs");
            if (!string.IsNullOrWhiteSpace(uhs)) return uhs;
        }
        return null;
    }

    private static long? ReadXErr(JsonDocument? doc)
    {
        if (doc is null) return null;
        var root = doc.RootElement;
        if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("XErr", out var value)) return null;
        if (value.ValueKind == JsonValueKind.Number && value.TryGetInt64(out var number)) return number;
        if (value.ValueKind == JsonValueKind.String && long.TryParse(value.GetString(), out var parsed)) return parsed;
        return null;
    }

    private static string? GetString(JsonElement root, string name)
        => root.ValueKind == JsonValueKind.Object
           && root.TryGetProperty(name, out var value)
           && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static int? GetInt(JsonElement root, string name)
    {
        if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty(name, out var value)) return null;
        if (value.ValueKind == JsonValueKind.Number && value.TryGetInt32(out var number)) return number;
        if (value.ValueKind == JsonValueKind.String && int.TryParse(value.GetString(), out var parsed)) return parsed;
        return null;
    }

    private readonly record struct JsonResponse(JsonDocument? Json, int StatusCode, bool IsSuccess);
}
