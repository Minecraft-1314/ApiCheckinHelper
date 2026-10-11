namespace ApiCheckinHelper.Core;

public static class Paths
{
    public const string ConfigName = "config.json";

    public static string AppDirectory { get; } = ResolveAppDirectory();

    public static string DataDirectory { get; private set; } = "";

    public static string ConfigPath { get; private set; } = "";

    private static string ResolveAppDirectory()
    {
        string? baseDirectory = AppContext.BaseDirectory;
        return string.IsNullOrEmpty(baseDirectory)
            ? Path.GetFullPath(AppDomain.CurrentDomain.BaseDirectory)
            : Path.GetFullPath(baseDirectory);
    }

    public static void Bind()
    {
        DataDirectory = ResolveDataDirectory();
        ConfigPath = Path.Combine(DataDirectory, ConfigName);
    }

    public static string ResolveDataDirectory()
    {
        foreach (string candidate in Candidates())
        {
            if (IsWritable(candidate))
            {
                return Path.GetFullPath(candidate);
            }
        }
        return Path.GetFullPath(Path.Combine(Path.GetTempPath(), "ApiCheckinHelper"));
    }

    private static IEnumerable<string> Candidates()
    {
        string? overridePath = Environment.GetEnvironmentVariable("CHECKIN_DATA_DIR");
        if (!string.IsNullOrWhiteSpace(overridePath))
        {
            yield return overridePath;
        }
        foreach (Environment.SpecialFolder folder in new[]
        {
            Environment.SpecialFolder.LocalApplicationData,
            Environment.SpecialFolder.ApplicationData,
        })
        {
            string root = Environment.GetFolderPath(folder);
            if (!string.IsNullOrEmpty(root))
            {
                yield return Path.Combine(root, "ApiCheckinHelper");
            }
        }
        string? profile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (!string.IsNullOrEmpty(profile))
        {
            yield return Path.Combine(profile, "ApiCheckinHelper");
        }
        yield return Path.Combine(Path.GetTempPath(), "ApiCheckinHelper");
    }

    private static bool IsWritable(string directory)
    {
        try
        {
            Directory.CreateDirectory(directory);
            string probe = Path.Combine(directory, ".write-probe-" + Environment.ProcessId);
            File.WriteAllText(probe, "1");
            File.Delete(probe);
            return true;
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or NotSupportedException or ArgumentException)
        {
            return false;
        }
    }

    public static void EnsureDataDirectory()
    {
        Directory.CreateDirectory(DataDirectory);
    }
}
