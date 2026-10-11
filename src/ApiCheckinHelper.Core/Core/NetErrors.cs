using System.Net.Http;
using System.Net.Sockets;
using System.Security.Authentication;

namespace ApiCheckinHelper.Core;

public static class NetErrors
{
    public static string OfflineState(NetError error)
    {
        return error.Kind switch
        {
            "dns" or "connect" or "refused" or "timeout" or "reset" => "offline",
            _ => "failed",
        };
    }

    public static NetError Classify(HttpRequestException error)
    {
        Exception? inner = error.InnerException;
        string text = error.Message + " " + (inner?.Message ?? "");
        if (inner is AuthenticationException)
        {
            return new NetError("证书校验失败，请检查系统时间或代理设置", "ssl");
        }
        if (inner is SocketException socket)
        {
            return socket.SocketErrorCode switch
            {
                SocketError.HostNotFound => new NetError("域名解析失败", "dns", true),
                SocketError.HostUnreachable => new NetError("无法连接到站点", "connect", true),
                SocketError.NetworkUnreachable => new NetError("网络不可达", "connect", true),
                SocketError.ConnectionRefused => new NetError("对方拒绝连接", "refused", true),
                SocketError.ConnectionReset => new NetError("连接被中断", "reset", true),
                SocketError.TimedOut => new NetError("连接超时", "timeout", true),
                _ => new NetError("无法连接到站点", "connect", true),
            };
        }
        if (text.Contains("SSL", StringComparison.OrdinalIgnoreCase))
        {
            return new NetError("证书校验失败，请检查系统时间或代理设置", "ssl");
        }
        if (text.Contains("Name or service not known", StringComparison.OrdinalIgnoreCase)
            || text.Contains("No such host", StringComparison.OrdinalIgnoreCase))
 {
            return new NetError("域名解析失败", "dns", true);
        }
        if (text.Contains("refused", StringComparison.OrdinalIgnoreCase))
 {
            return new NetError("对方拒绝连接", "refused", true);
        }
        return NetError.Offline();
    }
}
