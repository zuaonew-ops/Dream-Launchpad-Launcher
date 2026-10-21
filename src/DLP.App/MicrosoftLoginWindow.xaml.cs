using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows;
using DLP.App.Services;
using Microsoft.Web.WebView2.Core;

namespace DLP.App;

/// <summary>
/// 微软网页登录：内嵌 WebView2 打开微软登录页，走官方启动器同一个公共客户端，
/// 用户不需要注册任何 Azure 应用。拿到 access_token 后换 Minecraft 账号。
/// </summary>
public partial class MicrosoftLoginWindow : Window, INotifyPropertyChanged
{
    // 官方启动器/旧版 Live SDK 客户端，已对 Xbox Live 做过预授权，第三方可直接使用。
    private const string ClientId = "00000000402b5328";
    private const string RedirectUri = "https://login.live.com/oauth20_desktop.srf";
    private const string AuthorizeUrl =
        "https://login.live.com/oauth20_authorize.srf?client_id=" + ClientId +
        "&response_type=token&scope=XboxLive.signin%20offline_access&redirect_uri=" + RedirectUri;

    private readonly AccountManager _accounts;
    private readonly CancellationTokenSource _cts = new();
    private string _status = "正在打开微软登录页…";
    private bool _completed;

    public MicrosoftLoginWindow(AccountManager accounts)
    {
        InitializeComponent();
        _accounts = accounts;
        DataContext = this;
        Loaded += async (_, _) => await StartAsync();
        Closed += (_, _) => _cts.Cancel();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public string Status
    {
        get => _status;
        private set { _status = value; OnPropertyChanged(); }
    }

    private async Task StartAsync()
    {
        try
        {
            await Web.EnsureCoreWebView2Async();
            Web.CoreWebView2.NavigationStarting += OnNavigationStarting;
            Web.CoreWebView2.NavigationCompleted += OnNavigationCompleted;
            Web.CoreWebView2.SourceChanged += OnSourceChanged;
            Web.CoreWebView2.Settings.AreDevToolsEnabled = false;
            Web.CoreWebView2.Settings.IsStatusBarEnabled = false;
            Web.Source = new Uri(AuthorizeUrl);
        }
        catch (Exception ex)
        {
            Status = "无法启动内嵌浏览器（需要 WebView2 运行时）：" + ex.Message;
        }
    }

    private void OnNavigationStarting(object? sender, CoreWebView2NavigationStartingEventArgs e)
    {
        if (e.Uri.StartsWith(RedirectUri, StringComparison.OrdinalIgnoreCase))
            Status = "已授权，正在获取令牌…";
    }

    private async void OnNavigationCompleted(object? sender, CoreWebView2NavigationCompletedEventArgs e)
    {
        if (_completed) return;
        var uri = Web.CoreWebView2.Source;
        if (uri.StartsWith(RedirectUri, StringComparison.OrdinalIgnoreCase))
        {
            // 隐式流的 access_token 在 URL 片段里，WebView2 的 Source 不保证带片段，用脚本读回来。
            string hash = "";
            try { hash = await Web.CoreWebView2.ExecuteScriptAsync("location.hash + '|' + location.search"); } catch { }
            var raw = hash.Trim('"').Replace("\\u0026", "&");
            var token = Extract(raw, "access_token");
            if (string.IsNullOrWhiteSpace(token) && hash.Length > 0)
                token = Extract(uri, "access_token");
            if (!string.IsNullOrWhiteSpace(token)) await FinishAsync(token);
            else if (hash.Length > 2) Status = "授权页没有返回令牌，请点「重新加载」再试一次。";
        }
    }

    private async void OnSourceChanged(object? sender, CoreWebView2SourceChangedEventArgs e)
    {
        if (_completed) return;
        var uri = Web.CoreWebView2.Source;
        if (!uri.StartsWith(RedirectUri, StringComparison.OrdinalIgnoreCase)) return;

        string hash = "";
        try { hash = await Web.CoreWebView2.ExecuteScriptAsync("location.hash + '|' + location.search"); } catch { }
        var token = Extract(hash.Trim('"').Replace("\\u0026", "&"), "access_token");
        if (!string.IsNullOrWhiteSpace(token)) await FinishAsync(token);
    }

    private async Task FinishAsync(string accessToken)
    {
        _completed = true;
        Status = "登录成功，正在换取 Minecraft 账号…";
        try
        {
            var session = await _accounts.CompleteWebLoginAsync(accessToken, null, _cts.Token);
            Status = $"登录成功：{session.Account.Username}";
            await Task.Delay(600);
            DialogResult = true;
            Close();
        }
        catch (OperationCanceledException)
        {
            Status = "已取消";
        }
        catch (Exception ex)
        {
            Status = "登录失败：" + ex.Message;
            _completed = false;
        }
    }

    private static string? Extract(string text, string key)
    {
        var marker = key + "=";
        var index = text.IndexOf(marker, StringComparison.OrdinalIgnoreCase);
        if (index < 0) return null;
        index += marker.Length;
        var end = text.IndexOfAny(['&', '|', '#'], index);
        return end < 0 ? text[index..] : text[index..end];
    }

    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));

    private void Reload_Click(object sender, RoutedEventArgs e)
    {
        _completed = false;
        Web.Source = new Uri(AuthorizeUrl);
    }

    private void Cancel_Click(object sender, RoutedEventArgs e) => Close();
}
