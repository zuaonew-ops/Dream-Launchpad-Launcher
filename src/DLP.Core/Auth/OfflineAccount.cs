using System.Security.Cryptography;
using System.Text;

namespace DLP.Core.Auth;

public sealed record LaunchAccount(string Username, string Uuid, string AccessToken, string UserType)
{
    public static LaunchAccount Offline(string username)
    {
        var bytes = MD5.HashData(Encoding.UTF8.GetBytes("OfflinePlayer:" + username));
        bytes[6] = (byte)((bytes[6] & 0x0F) | 0x30); // version 3
        bytes[8] = (byte)((bytes[8] & 0x3F) | 0x80); // variant RFC 4122
        var uuid = Convert.ToHexString(bytes).ToLowerInvariant();
        return new LaunchAccount(username, uuid, "0", "legacy");
    }
}
