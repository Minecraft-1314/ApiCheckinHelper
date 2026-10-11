using System.Net;
using System.Net.Sockets;
using System.Globalization;

namespace ApiCheckinHelper.Core;

internal static class NetworkPolicy
{
    private const int MaxHostLength = 253;
    private const int MaxIpv6Bytes = 16;
    private const int Ipv4ByteCount = 4;

    private static readonly string[] AllowedSchemes = { "https", "http" };

    private static readonly HashSet<string> BlockedHosts = new(StringComparer.OrdinalIgnoreCase)
    {
        "0.0.0.0",
        "::",
        "::1",
        "169.254.169.254",
        "169.254.170.2",
        "metadata.google.internal",
        "metadata.goog",
        "localhost",
        "localhost.localdomain",
    };

    private static readonly string[] BlockedSuffixes =
    {
        ".localhost",
        ".local",
        ".internal",
        ".intranet",
        ".lan",
        ".home.arpa",
    };

    public static bool IsValidHost(string? host)
    {
        if (string.IsNullOrEmpty(host) || host.Length > MaxHostLength)
        {
            return false;
        }
        if (host.Length > 2 && host[0] == '[' && host[^1] == ']')
        {
            host = host[1..^1];
        }
        if (BlockedHosts.Contains(host) || BlockedSuffixes.Any(host.EndsWith))
        {
            return false;
        }
        if (host[0] == '.' || host[^1] == '.')
        {
            return false;
        }
        if (IPAddress.TryParse(host, out IPAddress? address))
        {
            return IsRoutable(address);
        }
        if (!host.Contains('.'))
        {
            return false;
        }
        foreach (string label in host.Split('.'))
        {
            if (label.Length is < 1 or > 63 || label[0] == '-' || label[^1] == '-')
            {
                return false;
            }
            foreach (char character in label)
            {
                bool allowed = character is >= 'a' and <= 'z'
                    or >= '0' and <= '9'
                    or '-';
                if (!allowed)
                {
                    return false;
                }
            }
        }
        return true;
    }

    public static string FormatHost(string host) =>
        host.Contains(':') && !host.StartsWith('[') ? $"[{host}]" : host;

    public static bool ValidateUrl(string? value, out Uri? uri)
    {
        uri = null;
        if (!Uri.TryCreate(value, UriKind.Absolute, out Uri? parsed)
            || !Array.Exists(AllowedSchemes, scheme => string.Equals(scheme, parsed.Scheme, StringComparison.Ordinal))
            || parsed.UserInfo.Length > 0)
        {
            return false;
        }
        string host = parsed.DnsSafeHost;
        if (!IsValidHost(host))
        {
            return false;
        }
        uri = parsed;
        return true;
    }

    public static string? ToPunycode(string host)
    {
        bool ascii = true;
        foreach (char character in host)
        {
            if (character >= 128)
            {
                ascii = false;
                break;
            }
        }
        if (ascii)
        {
            return host;
        }
        try
        {
            return new IdnMapping().GetAscii(host);
        }
        catch (ArgumentException)
        {
            return null;
        }
    }

    public static bool IsRoutable(IPAddress address)
    {
        return address.AddressFamily switch
        {
            AddressFamily.InterNetwork => IsPublicIPv4(address.GetAddressBytes()),
            AddressFamily.InterNetworkV6 => IsPublicIPv6(address),
            _ => false,
        };
    }

    private static bool IsPublicIPv4(byte[] bytes)
    {
        if (bytes.Length != Ipv4ByteCount)
        {
            return false;
        }
        if (bytes[0] is 0 or 10 or 127 or >= 224)
        {
            return false;
        }
        if (bytes[0] == 100 && bytes[1] is >= 64 and <= 127)
        {
            return false;
        }
        if (bytes[0] == 169 && bytes[1] == 254)
        {
            return false;
        }
        if (bytes[0] == 172 && bytes[1] is >= 16 and <= 31)
        {
            return false;
        }
        if (bytes[0] == 192 && bytes[1] == 168)
        {
            return false;
        }
        if (bytes[0] == 192 && bytes[1] == 0 && bytes[2] is 0 or 2)
        {
            return false;
        }
        if (bytes[0] == 192 && bytes[1] == 88 && bytes[2] == 99)
        {
            return false;
        }
        if (bytes[0] == 198 && bytes[1] is 18 or 19)
        {
            return false;
        }
        if (bytes[0] == 198 && bytes[1] == 51 && bytes[2] == 100)
        {
            return false;
        }
        if (bytes[0] == 203 && bytes[1] == 0 && bytes[2] == 113)
        {
            return false;
        }
        return true;
    }

    private static bool IsPublicIPv6(IPAddress address)
    {
        if (address.IsIPv4MappedToIPv6)
        {
            return IsPublicIPv4(address.MapToIPv4().GetAddressBytes());
        }
        byte[] bytes = address.GetAddressBytes();
        if (bytes.Length != MaxIpv6Bytes || IPAddress.IPv6Any.Equals(address) || IPAddress.IPv6Loopback.Equals(address))
        {
            return false;
        }
        if (bytes[0] == 0xFF || (bytes[0] & 0xFE) == 0xFC)
        {
            return false;
        }
        if (bytes[0] == 0xFE && (bytes[1] & 0xC0) is 0x80 or 0xC0)
        {
            return false;
        }
        if (IsDocumentationV6(bytes))
        {
            return false;
        }
        if (IsCompatibleV4(bytes) || IsNat64(bytes))
        {
            return IsPublicIPv4(bytes[12..]);
        }
        if (IsSixToFour(bytes))
        {
            return IsPublicIPv4(bytes[2..6]);
        }
        return !IsTeredo(bytes) || IsPublicIPv4(Invert(bytes[4..8]));
    }

    private static bool IsCompatibleV4(byte[] bytes)
    {
        for (int index = 0; index < 10; index++)
        {
            if (bytes[index] != 0)
            {
                return false;
            }
        }
        return bytes[10] != 0 || bytes[11] != 0;
    }

    private static bool IsNat64(byte[] bytes) =>
        bytes[0] == 0x00 && bytes[1] == 0x64 && bytes[2] == 0xFF && bytes[3] == 0x9B;

    private static bool IsSixToFour(byte[] bytes) => bytes[0] == 0x20 && bytes[1] == 0x02;

    private static bool IsTeredo(byte[] bytes) =>
        bytes[0] == 0x20 && bytes[1] == 0x01 && bytes[2] == 0x00 && bytes[3] == 0x00;

    private static bool IsDocumentationV6(byte[] bytes) =>
        bytes[0] == 0x20 && bytes[1] == 0x01 && bytes[2] == 0x0D && bytes[3] == 0xB8;

    private static byte[] Invert(byte[] bytes)
    {
        byte[] result = new byte[bytes.Length];
        for (int index = 0; index < bytes.Length; index++)
        {
            result[index] = (byte)~bytes[index];
        }
        return result;
    }
}
