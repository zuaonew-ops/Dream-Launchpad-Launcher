using System.Collections.Concurrent;
using System.Net.Http;
using Avalonia.Media.Imaging;
using DLP.Core.Infrastructure;

namespace DLP.Avalonia.Services;

/// <summary>网络图片缓存（Avalonia 版）：下载一次后复用同一个 Bitmap。</summary>
public sealed class ImageCache(HttpClient http, GamePaths paths, Log log)
{
    private readonly ConcurrentDictionary<string, Bitmap?> _memory = new();
    private readonly string _dir = Directory.CreateDirectory(Path.Combine(paths.Cache, "images")).FullName;

    public async Task<Bitmap?> GetAsync(string? url)
    {
        if (string.IsNullOrWhiteSpace(url)) return null;
        if (_memory.TryGetValue(url, out var cached)) return cached;

        var file = Path.Combine(_dir, Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(
            System.Text.Encoding.UTF8.GetBytes(url)))[..32] + ".img");

        try
        {
            if (!File.Exists(file))
            {
                using var response = await http.GetAsync(url);
                response.EnsureSuccessStatusCode();
                await using var stream = await response.Content.ReadAsStreamAsync();
                await using var output = File.Create(file);
                await stream.CopyToAsync(output);
            }

            await using var input = File.OpenRead(file);
            var bitmap = new Bitmap(input);
            _memory[url] = bitmap;
            return bitmap;
        }
        catch (Exception ex)
        {
            log.Warn($"图片加载失败：{url}（{ex.Message}）");
            _memory[url] = null;
            return null;
        }
    }
}
