using System.Diagnostics;
using System.Runtime.InteropServices;

namespace DLP.Avalonia.Services;

/// <summary>用系统默认方式打开文件夹/链接（Windows 用 explorer，Linux 用 xdg-open）。</summary>
public static class Shell
{
    public static void Open(string path)
    {
        try
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                Process.Start(new ProcessStartInfo(path) { UseShellExecute = true });
            else
                Process.Start("xdg-open", path);
        }
        catch
        {
            // 打不开就算了，不影响主流程
        }
    }
}
