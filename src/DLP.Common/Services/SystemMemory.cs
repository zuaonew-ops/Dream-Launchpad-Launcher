using System.Runtime.InteropServices;

namespace DLP.App.Services;

/// <summary>本机物理内存：给「内存上限」滑杆/输入框算真实上限用。</summary>
public static class SystemMemory
{
    /// <summary>物理内存总量（MB）。取不到时退回 GC 看到的可用内存。</summary>
    public static int TotalMb { get; } = DetectTotalMb();

    /// <summary>游戏可用的内存上限：留 1GB 给系统，再向下取整到 256MB，最少 1024MB。</summary>
    public static int MaxAllocatableMb { get; } = ComputeMax(TotalMb);

    /// <summary>给界面显示的说明文字，例如「本机 15.3 GB，上限 14336 MB」。</summary>
    public static string LimitText => $"本机 {TotalMb / 1024.0:0.#} GB，上限 {MaxAllocatableMb} MB";

    private static int ComputeMax(int totalMb)
    {
        var reserve = Math.Max(1024, totalMb / 16);
        var usable = Math.Max(1024, totalMb - reserve);
        return Math.Max(1024, usable / 256 * 256);
    }

    private static int DetectTotalMb()
    {
        try
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                var status = new MemoryStatusEx();
                if (GlobalMemoryStatusEx(status)) return (int)(status.ullTotalPhys / 1024 / 1024);
            }
            else
            {
                foreach (var line in File.ReadLines("/proc/meminfo"))
                {
                    if (!line.StartsWith("MemTotal:", StringComparison.Ordinal)) continue;
                    var kb = long.Parse(new string(line.Where(char.IsDigit).ToArray()));
                    return (int)(kb / 1024);
                }
            }
        }
        catch
        {
            // 读不到就走兜底
        }

        return Math.Clamp((int)(GC.GetGCMemoryInfo().TotalAvailableMemoryBytes / 1024 / 1024), 1024, 65536);
    }

    [StructLayout(LayoutKind.Sequential)]
    private class MemoryStatusEx
    {
        public uint dwLength = (uint)Marshal.SizeOf(typeof(MemoryStatusEx));
        public uint dwMemoryLoad;
        public ulong ullTotalPhys;
        public ulong ullAvailPhys;
        public ulong ullTotalPageFile;
        public ulong ullAvailPageFile;
        public ulong ullTotalVirtual;
        public ulong ullAvailVirtual;
        public ulong ullAvailExtendedVirtual;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GlobalMemoryStatusEx([In, Out] MemoryStatusEx lpBuffer);
}
