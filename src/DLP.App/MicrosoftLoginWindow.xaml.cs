using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Windows;
using DLP.App.Services;
using DLP.Core.Auth;

namespace DLP.App;

public partial class MicrosoftLoginWindow : Window, INotifyPropertyChanged
{
    private readonly AccountManager _accounts;
    private readonly Action? _openSettings;
    private readonly CancellationTokenSource _cts = new();
    private string _code = "……";
    private string _verificationUri = "";
    private string _status = "正在申请设备码…";
    private bool _browserOpened;
    private bool _showSettings;
    private bool _needsClientId;
    private string _clientIdInput = "";

    public MicrosoftLoginWindow(AccountManager accounts, Action? openSettings = null)
    {
        InitializeComponent();
        _accounts = accounts;
        _openSettings = openSettings;
        DataContext = this;
        Loaded += async (_, _) => await LoginAsync();
        Closed += (_, _) => _cts.Cancel();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public string Code
    {
        get => _code;
        private set { _code = value; OnPropertyChanged(); }
    }

    public string VerificationUri
    {
        get => _verificationUri;
        private set { _verificationUri = value; OnPropertyChanged(); OnPropertyChanged(nameof(HasVerificationUri)); }
    }

    public bool HasVerificationUri => !string.IsNullOrWhiteSpace(_verificationUri);

    /// <summary>没有可用客户端 ID 时显示注册引导。</summary>
    public bool NeedsClientId
    {
        get => _needsClientId;
        private set { _needsClientId = value; OnPropertyChanged(); }
    }

    public string ClientIdInput
    {
        get => _clientIdInput;
        set { _clientIdInput = value; OnPropertyChanged(); }
    }

    public bool ShowSettings
    {
        get => _showSettings;
        private set { _showSettings = value; OnPropertyChanged(); }
    }

    public string Status
    {
        get => _status;
        private set { _status = value; OnPropertyChanged(); }
    }

    private async Task LoginAsync()
    {
        if (_accounts.ResolveMicrosoftClientId() is null)
        {
            NeedsClientId = true;
            Code = "—";
            VerificationUri = "";
            Status = "还没有微软客户端 ID。注册一次即可（免费，约 3 分钟），之后所有用户都不用再填。";
            return;
        }

        try
        {
            var progress = new Progress<MicrosoftDeviceCode>(code =>
            {
                Code = code.UserCode;
                VerificationUri = code.VerificationUri;
                Status = "已自动打开浏览器，验证码已复制到剪贴板，粘贴即可完成登录。";
                try { Clipboard.SetText(code.UserCode); } catch { }
                if (!_browserOpened)
                {
                    _browserOpened = true;
                    OpenInBrowser(code.VerificationUri);
                }
            });

            var session = await _accounts.LoginMicrosoftAsync(progress, _cts.Token);
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
            // 只有内置客户端全部失败时才建议用户去填自己的 client_id。
            if (ex.Message.Contains("客户端 ID", StringComparison.Ordinal)) ShowSettings = true;
        }
    }

    private static void OpenInBrowser(string uri)
    {
        if (string.IsNullOrWhiteSpace(uri)) return;
        try { Process.Start(new ProcessStartInfo(uri) { UseShellExecute = true }); } catch { }
    }

    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));

    private void OpenBrowser_Click(object sender, RoutedEventArgs e) => OpenInBrowser(VerificationUri);

    private void OpenSettings_Click(object sender, RoutedEventArgs e)
    {
        Close();
        _openSettings?.Invoke();
    }

    private async void SaveClientId_Click(object sender, RoutedEventArgs e)
    {
        var id = ClientIdInput.Trim();
        if (id.Length < 8)
        {
            Status = "客户端 ID 看起来不对，复制 Azure 门户里的「应用程序(客户端) ID」再试。";
            return;
        }

        _accounts.SaveMicrosoftClientId(id);
        NeedsClientId = false;
        Status = "已保存，正在申请设备码…";
        await LoginAsync();
    }

    private void Cancel_Click(object sender, RoutedEventArgs e) => Close();
}
