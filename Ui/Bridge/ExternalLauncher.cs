using System.Diagnostics;
using System.IO;
using ApiCheckinHelper.Core;

namespace ApiCheckinHelper.Ui.Bridge;

public static class ExternalLauncher
{
    public static bool Open(string? target)
    {
        if (string.IsNullOrWhiteSpace(target))
        {
            return false;
        }
        string value = target.Trim();
        if (OpenDataDirectory(value))
        {
            return true;
        }
        return Uri.TryCreate(value, UriKind.Absolute, out Uri? uri)
            && uri.Scheme is "http" or "https"
            && uri.UserInfo.Length == 0
            && Model.IsValidHost(uri.DnsSafeHost)
            && Start(uri.AbsoluteUri, out _);
    }

    public static bool OpenDataDirectory(string? target)
    {
        string directory = target ?? ConfigStore.DataDirectory;
        if (string.IsNullOrWhiteSpace(directory) || !Directory.Exists(directory))
        {
            AppLog.Warn("数据目录不存在：" + directory);
            return false;
        }
        return Start(Path.GetFullPath(directory), out _);
    }

    private static bool Start(string target, out string error)
    {
        try
        {
            Process.Start(new ProcessStartInfo(target) { UseShellExecute = true });
            error = "";
            return true;
        }
        catch (Exception failure) when (
            failure is System.ComponentModel.Win32Exception
                or InvalidOperationException
                or FileNotFoundException
                or ArgumentException
                or NotSupportedException)
        {
            error = failure.Message;
            AppLog.Error("无法打开外部目标", failure.ToString());
            return false;
        }
    }
}
