using System.Security.Cryptography;
using DLP.Core.Infrastructure;

namespace DLP.Core.Download;

public sealed class DownloadException(string message, Exception? inner = null) : Exception(message, inner);

/// <summary>单文件下载：存在且校验通过则跳过；失败重试；SHA1 校验后原子替换。</summary>
public sealed class DownloadClient(HttpClient http, Log log)
{
    private const int MaxAttempts = 3;

    public async Task DownloadAsync(string url, string destination, string? sha1 = null, long? size = null, CancellationToken ct = default)
    {
        url = Mirrors.Rewrite(url);
        if (IsValid(destination, sha1, size)) return;

        Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
        var temp = destination + ".dlp-part";

        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                using var response = await http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct);
                response.EnsureSuccessStatusCode();
                await using (var input = await response.Content.ReadAsStreamAsync(ct))
                await using (var output = File.Create(temp))
                    await input.CopyToAsync(output, ct);

                if (sha1 is not null && !string.Equals(HashFile(temp), sha1, StringComparison.OrdinalIgnoreCase))
                    throw new DownloadException($"SHA1 校验失败：{destination}");

                File.Move(temp, destination, overwrite: true);
                return;
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                TryDelete(temp);
                if (attempt == MaxAttempts)
                    throw new DownloadException($"下载失败（{attempt} 次）：{url}\n{ex.Message}", ex);
                log.Warn($"下载重试 {attempt}/{MaxAttempts - 1}：{url}（{ex.Message}）");
                await Task.Delay(TimeSpan.FromSeconds(Math.Pow(2, attempt - 1)), ct);
            }
        }
    }

    public static bool IsValid(string path, string? sha1, long? size)
    {
        if (!File.Exists(path)) return false;
        if (size is { } expectedSize && new FileInfo(path).Length != expectedSize) return false;
        if (sha1 is null) return true;
        return string.Equals(HashFile(path), sha1, StringComparison.OrdinalIgnoreCase);
    }

    public static string HashFile(string path)
    {
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(SHA1.HashData(stream)).ToLowerInvariant();
    }

    private static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { /* 忽略 */ }
    }
}
