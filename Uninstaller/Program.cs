using System.Diagnostics;
using System.Text.RegularExpressions;
using Microsoft.Win32;

namespace ApiCheckinHelper.Uninstaller;

internal static partial class Program
{
    private const string ProductName = "API 签到助手";

    [GeneratedRegex(@"(?i)\bmsiexec(?:\.exe)?\s+/[ix]\s*\{([0-9a-f-]{36})\}")]
    private static partial Regex ProductCodePattern();

    private sealed record InstalledProduct(string UninstallString, string? QuietUninstallString, Guid? ProductCode);

    [STAThread]
    private static int Main(string[] args)
    {
        bool checkOnly = HasFlag(args, "--check");
        bool quiet = HasFlag(args, "/quiet", "--quiet");
        try
        {
            InstalledProduct[] products = FindProducts();
            if (checkOnly)
            {
                return products.Length == 0 ? 2 : 0;
            }
            if (products.Length == 0)
            {
                return 2;
            }
            foreach (InstalledProduct product in products)
            {
                int exitCode = RunUninstall(product, quiet);
                if (exitCode != 0 && exitCode != 3010)
                {
                    return exitCode;
                }
            }
            return 0;
        }
        catch (Exception)
        {
            return 5;
        }
    }

    private static bool HasFlag(string[] args, params string[] flags)
    {
        return args.Any(arg => flags.Contains(arg, StringComparer.OrdinalIgnoreCase));
    }

    private static InstalledProduct[] FindProducts()
    {
        List<InstalledProduct> products = [];
        foreach ((RegistryHive hive, RegistryView view) in new[]
        {
            (RegistryHive.LocalMachine, RegistryView.Registry64),
            (RegistryHive.LocalMachine, RegistryView.Registry32),
            (RegistryHive.CurrentUser, RegistryView.Default),
        })
        {
            try
            {
                using RegistryKey baseKey = RegistryKey.OpenBaseKey(hive, view);
                using RegistryKey? uninstall = baseKey.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall");
                if (uninstall is null)
                {
                    continue;
                }
                foreach (string subKeyName in uninstall.GetSubKeyNames())
                {
                    using RegistryKey? item = uninstall.OpenSubKey(subKeyName);
                    if (item is null || !ProductName.Equals(item.GetValue("DisplayName") as string, StringComparison.Ordinal))
                    {
                        continue;
                    }
                    string command = item.GetValue("UninstallString") as string ?? "";
                    string? quietCommand = item.GetValue("QuietUninstallString") as string;
                    if (command.Length == 0)
                    {
                        continue;
                    }
                    Guid? productCode = null;
                    Match match = ProductCodePattern().Match(command);
                    if (match.Success && Guid.TryParse(match.Groups[1].Value, out Guid parsed))
                    {
                        productCode = parsed;
                    }
                    products.Add(new InstalledProduct(command, quietCommand, productCode));
                }
            }
            catch (Exception error)
            {
                System.Diagnostics.Debug.WriteLine(error);
            }
        }
        return products.Distinct().ToArray();
    }

    private static int RunUninstall(InstalledProduct product, bool quiet)
    {
        string fileName;
        string arguments;
        if (product.ProductCode is Guid productCode)
        {
            fileName = "msiexec.exe";
            arguments = quiet
                ? $"/x \"{productCode:B}\" /qn /norestart"
                : $"/x \"{productCode:B}\" /passive /norestart";
        }
        else
        {
            string command = quiet && !string.IsNullOrWhiteSpace(product.QuietUninstallString)
                ? product.QuietUninstallString
                : product.UninstallString;
            (fileName, arguments) = SplitCommand(Environment.ExpandEnvironmentVariables(command));
            if (quiet && !Regex.IsMatch(arguments, @"(?i)(?:^|\s)/(?:VERYSILENT|S)(?:\s|$)"))
            {
                arguments = AppendArguments(arguments, "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART");
            }
        }
        ProcessStartInfo startInfo = new()
        {
            FileName = fileName,
            Arguments = arguments,
            UseShellExecute = true,
        };
        using Process? process = Process.Start(startInfo);
        if (process is null)
        {
            return 3;
        }
        process.WaitForExit();
        return process.ExitCode;
    }

    private static (string FileName, string Arguments) SplitCommand(string command)
    {
        string value = command.Trim();
        if (value.StartsWith('"'))
        {
            int closingQuote = value.IndexOf('"', 1);
            if (closingQuote > 0)
            {
                return (value[1..closingQuote], value[(closingQuote + 1)..].TrimStart());
            }
        }
        int separator = value.IndexOfAny([' ', '\t']);
        return separator < 0
            ? (value, "")
            : (value[..separator], value[(separator + 1)..].TrimStart());
    }

    private static string AppendArguments(string current, string addition)
    {
        return string.IsNullOrWhiteSpace(current) ? addition : current + " " + addition;
    }
}
