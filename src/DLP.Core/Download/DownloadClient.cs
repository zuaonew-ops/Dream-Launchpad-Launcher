using System.Net;
using System.Security.Cryptography;
using DLP.Core.Infrastructure;

namespace DLP.Core.Download;

public sealed class DownloadException(string message, Exception? inner = null) : Exception(message, inner);

/// <summary>单文件下载：存在且校验通过则跳过；失败换源重试；大文件多连接分段下载；SHA1 校验后原子替换。</summary>
public sealed class DownloadClient(HttpClient http, Log log)
{
    private const int MaxAttempts = 3;

    /// <summary>小于这个大小直接单连接下载，省掉分段协商的开销。</summary>
    private const long RangeThreshold = 4L * 1024 * 1024;

    /// <summary>大文件的分段数；CDN 对单连接限速时靠并发把带宽吃满。</summary>
    private const int MaxParts = 6;

    /// <summary>单次请求的基础超时，实际按文件大小放大，避免大文件被误杀。</summary>
    private static readonly TimeSpan BaseAttemptTimeout = TimeSpan.FromSeconds(20);

    public async Task DownloadAsync(string url, string destination, string? sha1 = null, long? size = null, CancellationToken ct = default,
        bool allowRange = true)
    {
        if (IsValid(destination, sha1, size)) return;

        Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
        var temp = destination + ".dlp-part";
        var current = Mirrors.Rewrite(url);

        for (var attempt = 1; attempt <= MaxAttempts; attempt++)
        {
            try
            {
                await TransferAsync(current, temp, size, ct, allowRange);
                if (sha1 is not null && !string.Equals(HashFile(temp), sha1, StringComparison.OrdinalIgnoreCase))
                    throw new DownloadException($"SHA1 校验失败：{destination}");

                File.Move(temp, destination, overwrite: true);
                return;
            }
            catch (Exception ex)
            {
                // 外部取消才算真取消；单次超时（我们自己的 CancellationTokenSource）要走换源重试。
                if (ex is OperationCanceledException && ct.IsCancellationRequested) throw;

                Cleanup(temp);

                // 第一次失败就直接换另一个源（镜像/官方互为备份），别在慢源上反复耗时间。
                var alternative = Mirrors.Alternative(url, current);
                if (attempt < MaxAttempts && alternative is not null)
                {
                    log.Warn($"下载失败，换源重试：{current} -> {alternative}（{ex.Message}）");
                    current = alternative;
                    continue;
                }

                if (attempt == MaxAttempts)
                    throw new DownloadException($"下载失败（{attempt} 次）：{current}\n{ex.Message}", ex);

                log.Warn($"下载重试 {attempt}/{MaxAttempts - 1}：{current}（{ex.Message}）");
                await Task.Delay(TimeSpan.FromMilliseconds(400 * attempt), ct);
            }
        }
    }

    private Task TransferAsync(string url, string temp, long? size, CancellationToken ct, bool allowRange)
        => allowRange && size is { } length && length >= RangeThreshold
            ? DownloadInPartsAsync(url, temp, length, ct)
            : DownloadSingleAsync(url, temp, size, ct);

    private async Task DownloadSingleAsync(string url, string temp, long? size, CancellationToken ct)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeout.CancelAfter(TimeoutFor(size ?? 0));

        using var response = await http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, timeout.Token);
        response.EnsureSuccessStatusCode();
        await using var input = await response.Content.ReadAsStreamAsync(timeout.Token);
        await using var output = File.Create(temp);
        await input.CopyToAsync(output, timeout.Token);
    }

    /// <summary>分段下载：任何一段失败（不支持 Range、长度不符、超时）都整体退回单连接，保证正确性。</summary>
    private async Task DownloadInPartsAsync(string url, string temp, long size, CancellationToken ct)
    {
        try
        {
            await DownloadPartsCoreAsync(url, temp, size, ct);
        }
        catch (Exception ex) when (ex is not OperationCanceledException || !ct.IsCancellationRequested)
        {
            log.Warn($"分段下载不可用，改用单连接：{Path.GetFileName(temp)}（{ex.Message}）");
            Cleanup(temp);
            await DownloadSingleAsync(url, temp, size, ct);
        }
    }

    private async Task DownloadPartsCoreAsync(string url, string temp, long size, CancellationToken ct)
    {
        var partCount = (int)Math.Clamp((size + RangeThreshold - 1) / RangeThreshold, 2, MaxParts);
        var chunk = size / partCount;
        var partFiles = new string[partCount];

        try
        {
            var tasks = new Task[partCount];
            for (var i = 0; i < partCount; i++)
            {
                var index = i;
                var start = index * chunk;
                var end = index == partCount - 1 ? size - 1 : start + chunk - 1;
                partFiles[index] = $"{temp}.{index}";
                tasks[index] = DownloadPartAsync(url, partFiles[index], start, end, size, ct);
            }

            await Task.WhenAll(tasks);

            await using (var output = File.Create(temp))
            {
                foreach (var part in partFiles)
                {
                    await using var input = File.OpenRead(part);
                    await input.CopyToAsync(output, ct);
                }
            }
        }
        finally
        {
            foreach (var part in partFiles)
                if (part is not null) TryDelete(part);
        }
    }

    private async Task DownloadPartAsync(string url, string path, long start, long end, long totalSize, CancellationToken ct)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(ct);
        timeout.CancelAfter(TimeoutFor(end - start + 1));

        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.Range = new System.Net.Http.Headers.RangeHeaderValue(start, end);

        using var response = await http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, timeout.Token);
        if (response.StatusCode != HttpStatusCode.PartialContent)
            throw new DownloadException("服务器不支持分段下载");

        await using var input = await response.Content.ReadAsStreamAsync(timeout.Token);
        await using var output = File.Create(path);
        await input.CopyToAsync(output, timeout.Token);

        if (new FileInfo(path).Length != end - start + 1)
            throw new DownloadException("分段长度不匹配");
    }

    /// <summary>超时按大小放大：每 MB 给 3 秒，20 秒起步，最多 3 分钟。</summary>
    private static TimeSpan TimeoutFor(long bytes)
    {
        var seconds = 60 + bytes / (1024.0 * 1024.0) * 3;
        return TimeSpan.FromSeconds(Math.Clamp(seconds, 60, 240));
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

    private void Cleanup(string temp)
    {
        TryDelete(temp);
        foreach (var part in Directory.EnumerateFiles(Path.GetDirectoryName(temp)!, Path.GetFileName(temp) + ".*"))
            TryDelete(part);
    }

    private void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { /* 忽略 */ }
    }
}
