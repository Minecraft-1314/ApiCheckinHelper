using ApiCheckinHelper.Core.Json;
using System.IO;
using System.Globalization;
using System.Buffers;
using System.Text;

namespace ApiCheckinHelper.Core;

public static class Model
{
    public const int StateVersion = 1;
    public const int HistoryLimit = 30;
    public const int DefaultQuotaPerUnit = 500000;
    public const int DefaultTimeoutMs = 15000;
    public const int DefaultConcurrency = 3;
    public const int ConcurrencyMin = 1;
    public const int ConcurrencyMax = 6;
    public const int TimeoutMinMs = 5000;
    public const int TimeoutMaxMs = 60000;
    public const int MaxNameLength = 80;
    public const int MaxEntryLength = 2048;
    public const int MaxTokenLength = 4096;
    public const int MaxTagCount = 8;
    public const int MaxTagLength = 80;
    public const long MaxConfigBytes = 4 * 1024 * 1024;
    public const int MaxImportEntries = 100;

    public static readonly string[] CredentialTypes = { "token" };
    public static readonly string[] Themes = { "auto", "light", "dark" };
    public static readonly string[] Languages = { "auto", "zh-cn", "en-us" };

    private static readonly string[] AllowedSchemes = { "https", "http" };

    private static readonly SearchValues<char> UrlBreaks = SearchValues.Create("#?");

    public static string Slugify(string? host)
    {
        StringBuilder builder = new();
        bool pendingDash = false;
        foreach (char raw in Coerce.ToText(host))
        {
            char ch = char.ToLowerInvariant(raw);
            if ((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9'))
            {
                if (pendingDash && builder.Length > 0)
                {
                    builder.Append('-');
                }
                pendingDash = false;
                builder.Append(ch);
            }
            else
            {
                pendingDash = true;
            }
        }
        return builder.ToString();
    }

    public static string? ToPunycode(string host) => NetworkPolicy.ToPunycode(host);

    public static bool IsValidHost(string? host) => NetworkPolicy.IsValidHost(host);

    public static string FormatHost(string host) => NetworkPolicy.FormatHost(host);

    public static (string Host, string Scheme)? SplitHost(object? raw)
    {
        string text = Coerce.ToText(raw).Trim().ToLowerInvariant();
        if (text.Length == 0)
        {
            return null;
        }
        string scheme = "https";
        int schemeIndex = text.IndexOf("://", StringComparison.Ordinal);
        if (schemeIndex >= 0)
        {
            scheme = text[..schemeIndex].Trim();
            text = text[(schemeIndex + 3)..];
        }
        if (!Array.Exists(AllowedSchemes, item => string.Equals(item, scheme, StringComparison.Ordinal)))
        {
            return null;
        }
        int cut = text.AsSpan().IndexOfAny(UrlBreaks);
        if (cut >= 0)
        {
            text = text[..cut];
        }
        int at = text.LastIndexOf('@');
        if (at >= 0)
        {
            text = text[(at + 1)..];
        }
        int slash = text.IndexOf('/');
        if (slash >= 0)
        {
            text = text[..slash];
        }
        if (text.StartsWith('['))
        {
            int close = text.IndexOf(']');
            if (close < 0)
            {
                return null;
            }
            string inner = ToPunycode(text[1..close]) ?? "";
            return IsValidHost(inner) ? (inner, scheme) : null;
        }
        string host = text;
        int colon = text.LastIndexOf(':');
        if (colon >= 0)
        {
            string port = text[(colon + 1)..];
            if (port.Length == 0 || !port.All(char.IsDigit))
            {
                return null;
            }
            host = text[..colon];
        }
        string? ascii = ToPunycode(host);
        if (ascii is null || !IsValidHost(ascii))
        {
            return null;
        }
        return (ascii, scheme);
    }

    public static string NormalizeEntry(object? raw)
    {
        string entry = Coerce.ToText(raw, "/").Trim();
        if (entry.Length == 0)
        {
            return "/";
        }
        string normalized = entry.StartsWith('/') ? entry : "/" + entry;
        return normalized.Length <= MaxEntryLength ? normalized : normalized[..MaxEntryLength];
    }

    public static SiteSnapshot NormalizeSnapshot(SiteSnapshot? source)
    {
        source ??= new SiteSnapshot();
        string version = Coerce.ToText(source.Version).Trim();
        string displayType = Coerce.ToText(source.DisplayType, "USD").Trim().ToUpperInvariant();
        return new SiteSnapshot
        {
            SystemName = Coerce.ToText(source.SystemName),
            Version = version.Length > 0 ? version : null,
            CheckinEnabled = source.CheckinEnabled,
            Turnstile = source.Turnstile,
            QuotaPerUnit = Coerce.ToInt(
                source.QuotaPerUnit == FlexibleIntConverter.Unparsable ? DefaultQuotaPerUnit : source.QuotaPerUnit,
                DefaultQuotaPerUnit,
                1,
                int.MaxValue),
            DisplayInCurrency = source.DisplayInCurrency,
            DisplayType = displayType.Length > 0 ? displayType[..Math.Min(12, displayType.Length)] : "USD",
            ExchangeRate = Coerce.ToFloat(
                source.ExchangeRate == FlexibleDoubleConverter.Unparsable ? 1d : source.ExchangeRate,
                1d,
                0.000001d,
                1_000_000d),
        };
    }

    public static Settings NormalizeSettings(Settings? source)
    {
        source ??= new Settings();
        string theme = Coerce.ToText(source.Theme, "auto").Trim().ToLowerInvariant();
        string language = Coerce.ToText(source.Language, "auto").Trim().ToLowerInvariant();
        if (language is "zh" or "zh-cn" or "zh-hans") language = "zh-cn";
        if (language is "en" or "en-us" or "en-gb") language = "en-us";
        return new Settings
        {
            Concurrency = Coerce.ToInt(
                source.Concurrency == FlexibleIntConverter.Unparsable ? DefaultConcurrency : source.Concurrency,
                DefaultConcurrency,
                ConcurrencyMin,
                ConcurrencyMax),
            TimeoutMs = Coerce.ToInt(
                source.TimeoutMs == FlexibleIntConverter.Unparsable ? DefaultTimeoutMs : source.TimeoutMs,
                DefaultTimeoutMs,
                TimeoutMinMs,
                TimeoutMaxMs),
            Notify = source.Notify,
            SkipManualOnly = source.SkipManualOnly,
            Theme = Array.IndexOf(Themes, theme) >= 0 ? theme : "auto",
            Language = Array.IndexOf(Languages, language) >= 0 ? language : "auto",
        };
    }

    public static Credential NormalizeCredential(Credential? source)
    {
        source ??= new Credential();
        string type = Coerce.ToText(source.Type);
        if (Array.IndexOf(CredentialTypes, type) < 0)
        {
            type = "token";
        }
        string token = Coerce.ToText(source.Token);
        return new Credential
        {
            Type = type,
            Token = Clamp(token.Trim(), MaxTokenLength),
            UserId = Coerce.ToInt(source.UserId, 0, 0, 1_000_000_000)
        };
    }

    public static Site? NormalizeSite(Site? source)
    {
        if (source is null)
        {
            return null;
        }
        object? hostRaw = source.Host;
        if (string.IsNullOrWhiteSpace(Coerce.ToText(hostRaw)))
        {
            hostRaw = source.Origin;
        }
        (string Host, string Scheme)? parsed = SplitHost(hostRaw);
        if (parsed is null)
        {
            return null;
        }
        string entry = NormalizeEntry(source.Entry);
        string origin = $"{parsed.Value.Scheme}://{NetworkPolicy.FormatHost(parsed.Value.Host)}";
        string rawId = Coerce.ToText(source.Id).Trim().ToLowerInvariant();
        string name = Clamp(Coerce.ToText(source.Name).Trim(), MaxNameLength);
        return new Site
        {
            Id = rawId.Length > 0 ? rawId : Slugify(parsed.Value.Host),
            Name = name.Length > 0 ? name : parsed.Value.Host,
            Host = parsed.Value.Host,
            Origin = origin,
            Entry = entry,
            Tags = (source.Tags ?? new List<string>())
                .Where(tag => !string.IsNullOrWhiteSpace(tag))
                .Take(MaxTagCount)
                .Select(tag => Clamp(tag.Trim(), MaxTagLength))
                .ToList(),
            Bookmark = origin + entry,
            Snapshot = NormalizeSnapshot(source.Snapshot),
        };
    }

    public static List<Site> NormalizeSiteList(IEnumerable<Site>? raw)
    {
        List<Site> result = new();
        if (raw is null)
        {
            return result;
        }
        HashSet<string> seen = new(StringComparer.Ordinal);
        foreach (Site? item in raw)
        {
            Site? site = NormalizeSite(item);
            if (site is null || !seen.Add(site.Id) || !seen.Add(site.Host))
            {
                continue;
            }
            result.Add(site);
        }
        return result;
    }

    public static RunResult? NormalizeResult(RunResult? source)
    {
        if (source is null)
        {
            return null;
        }
        return new RunResult
        {
            Id = Coerce.ToText(source.Id),
            State = Coerce.ToText(source.State, "failed"),
            Message = Coerce.ToText(source.Message),
            Quota = Coerce.ToLong(source.Quota, 0, long.MinValue, long.MaxValue),
            Path = Coerce.ToText(source.Path),
            HttpStatus = Coerce.ToInt(source.HttpStatus, 0, 0, 1000),
            At = Coerce.ToLong(source.At, 0, 0, long.MaxValue),
        };
    }

    public static HistoryBucket NormalizeHistoryBucket(HistoryBucket? source)
    {
        source ??= new HistoryBucket();
        return new HistoryBucket
        {
            Total = Coerce.ToInt(source.Total, 0, 0, int.MaxValue),
            Success = Coerce.ToInt(source.Success, 0, 0, int.MaxValue),
            Already = Coerce.ToInt(source.Already, 0, 0, int.MaxValue),
            Failed = Coerce.ToInt(source.Failed, 0, 0, int.MaxValue),
            Skipped = Coerce.ToInt(source.Skipped, 0, 0, int.MaxValue),
            Quota = Coerce.ToLong(source.Quota, 0, 0, long.MaxValue),
            QuotaUsd = Coerce.ToFloat(source.QuotaUsd, 0d, 0d, 1_000_000_000_000d),
            At = Coerce.ToLong(source.At, 0, 0, long.MaxValue),
        };
    }

    public static VerifySummary NormalizeVerify(VerifySummary? source)
    {
        source ??= new VerifySummary();
        return new VerifySummary
        {
            At = Coerce.ToLong(source.At, 0, 0, long.MaxValue),
            Total = Coerce.ToInt(source.Total, 0, 0, 1_000_000),
            Ok = Coerce.ToInt(source.Ok, 0, 0, 1_000_000),
            Failed = Coerce.ToInt(source.Failed, 0, 0, 1_000_000),
            Items = (source.Items ?? new List<VerifyEntry>())
                .Where(item => item is not null)
                .Select(item => new VerifyEntry
                {
                    Id = Coerce.ToText(item.Id),
                    Name = Coerce.ToText(item.Name),
                    Host = Coerce.ToText(item.Host),
                    Ok = item.Ok,
                    Message = Coerce.ToText(item.Message),
                })
                .ToList(),
        };
    }

    public static Dictionary<string, HistoryBucket> PruneHistory(Dictionary<string, HistoryBucket> history)
    {
        if (history.Count <= HistoryLimit)
        {
            return history;
        }
        Dictionary<string, HistoryBucket> result = new(StringComparer.Ordinal);
        foreach (string day in history.Keys.OrderByDescending(key => key, StringComparer.Ordinal).Take(HistoryLimit))
        {
            result[day] = history[day];
        }
        return result;
    }

    public static bool IsConfigured(Credential? credential)
    {
        if (credential is null)
        {
            return false;
        }
        return credential.Type == "token" && !string.IsNullOrWhiteSpace(credential.Token);
    }

    public static string FormatStamp(long milliseconds)
    {
        return DateTimeOffset.FromUnixTimeMilliseconds(milliseconds).ToLocalTime()
            .ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
    }

    private static string Clamp(string value, int maximumLength) =>
        value.Length <= maximumLength ? value : value[..maximumLength];
}
