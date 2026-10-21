using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using DLP.Core.Infrastructure;

namespace DLP.App.Services;

/// <summary>下载并缓存网络图片（模组图标、整合包封面/截图），内存 + 磁盘两级缓存。</summary>
public sealed class ImageCache(HttpClient http, GamePaths paths)
{
    private readonly Dictionary<string, ImageSource?> _memory = new(StringComparer.OrdinalIgnoreCase);
    private readonly SemaphoreSlim _gate = new(1, 1);

    private string CacheDir => Path.Combine(paths.Cache, "images");

    public async Task<ImageSource?> LoadAsync(string? url, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(url)) return null;
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) ||
            (uri.Scheme != Uri.UriSchemeHttps && uri.Scheme != Uri.UriSchemeHttp)) return null;

        await _gate.WaitAsync(ct);
        ImageSource? cached;
        try
        {
            if (_memory.TryGetValue(url, out cached)) return cached;
        }
        finally
        {
            _gate.Release();
        }

        var image = await LoadFromDiskOrDownloadAsync(url, ct);
        await _gate.WaitAsync(ct);
        try
        {
            _memory[url] = image;
        }
        finally
        {
            _gate.Release();
        }

        return image;
    }

    private async Task<ImageSource?> LoadFromDiskOrDownloadAsync(string url, CancellationToken ct)
    {
        try
        {
            Directory.CreateDirectory(CacheDir);
            var file = Path.Combine(CacheDir, Hash(url) + ".img");
            if (!File.Exists(file))
            {
                using var response = await http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct);
                if (!response.IsSuccessStatusCode) return null;
                var bytes = await response.Content.ReadAsByteArrayAsync(ct);
                if (bytes.Length == 0) return null;
                await File.WriteAllBytesAsync(file, bytes, ct);
            }

            return Decode(file);
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or OperationCanceledException)
        {
            return null;
        }
    }

    private static ImageSource? Decode(string file)
    {
        try
        {
            using var stream = File.OpenRead(file);
            var bitmap = new BitmapImage();
            bitmap.BeginInit();
            bitmap.CacheOption = BitmapCacheOption.OnLoad;
            bitmap.StreamSource = stream;
            bitmap.EndInit();
            bitmap.Freeze();
            return bitmap;
        }
        catch
        {
            return null;
        }
    }

    private static string Hash(string text)
        => Convert.ToHexString(SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(text)))[..24].ToLowerInvariant();
}
