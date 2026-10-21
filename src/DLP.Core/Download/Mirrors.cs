// 作者：zuao
namespace DLP.Core.Download;

/// <summary>
/// BMCLAPI 镜像改写：把 Mojang 官方下载地址替换为 <see cref="BaseUrl"/>。
/// 未匹配的地址原样返回；<see cref="Enabled"/> 为 false 时全部原样返回。
/// </summary>
public static class Mirrors
{
    /// <summary>镜像站根地址（结尾不带斜杠）。</summary>
    public const string BaseUrl = "https://bmclapi2.bangbang93.com";

    /// <summary>官方域名前缀 -> 镜像地址前缀；按顺序匹配，命中即返回。</summary>
    private static readonly (string Prefix, string Replacement)[] Rules =
    [
        ("https://libraries.minecraft.net/", BaseUrl + "/maven/"),
        ("https://resources.download.minecraft.net/", BaseUrl + "/assets/"),
        ("https://piston-meta.mojang.com/", BaseUrl + "/"),
        ("https://launchermeta.mojang.com/", BaseUrl + "/"),
        ("https://launcher.mojang.com/", BaseUrl + "/")
    ];

    /// <summary>镜像总开关；false 时 <see cref="Rewrite"/> 不做任何改动（默认走官方，实测官方 CDN 更快）。</summary>
    public static bool Enabled { get; set; }

    /// <summary>给出同一文件的另一个下载源：当前用镜像就返回官方地址，反之返回镜像地址；没有对应源时返回 null。</summary>
    public static string? Alternative(string officialUrl, string currentUrl)
    {
        if (string.IsNullOrEmpty(officialUrl)) return null;

        if (!string.Equals(currentUrl, officialUrl, StringComparison.OrdinalIgnoreCase))
            return officialUrl;

        foreach (var (prefix, replacement) in Rules)
        {
            if (!officialUrl.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) continue;
            var mirrored = replacement + officialUrl[prefix.Length..];
            return string.Equals(mirrored, currentUrl, StringComparison.OrdinalIgnoreCase) ? null : mirrored;
        }

        return null;
    }

    /// <summary>把官方下载地址改写为镜像地址；不认识的地址原样返回。</summary>
    public static string Rewrite(string url)
    {
        if (!Enabled || string.IsNullOrEmpty(url)) return url;

        foreach (var (prefix, replacement) in Rules)
        {
            if (!url.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) continue;
            return replacement + url[prefix.Length..];
        }

        return url;
    }
}
