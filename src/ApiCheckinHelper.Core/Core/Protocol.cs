using System.Collections.Concurrent;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public sealed class StatusResult
{
    public string State { get; init; } = "offline";

    public string Message { get; init; } = "";

    public SiteSnapshot? Meta { get; init; }

    public int HttpStatus { get; set; }

    public string Origin { get; init; } = "";
}

public sealed class IdentityResult
{
    public string State { get; init; } = "unauthorized";

    public string Message { get; init; } = "";

    public int UserId { get; init; }

    public string Account { get; init; } = "";
}

public sealed class CheckinVerdict
{
    public string State { get; init; } = "failed";

    public string Message { get; init; } = "";

    public long Quota { get; init; }

    public string Date { get; init; } = "";

    public string Path { get; set; } = "";

    public int HttpStatus { get; set; }
}

public static partial class Protocol
{
    public const long StatusTtlMs = 15 * 60 * 1000;
    public const string RefreshCookie = "new_api_refresh";
    public const long ExpiryMarginMs = 60000;
    public const int DefaultQuotaPerUnit = 500000;

    public static readonly string[] CheckinPaths =
    {
        "/api/user/checkin",
        "/api/user/clock_in",
        "/api/user/check_in",
        "/api/user/sign_in",
        "/api/user/daily_checkin",
        "/api/user/attendance",
    };

    public static readonly string[] SelfPaths = { "/api/user/self", "/api/user/token" };

    private static readonly string[] MessageKeys = { "message", "msg", "error", "detail" };

    [GeneratedRegex("turnstile|人机验证|请完成验证", RegexOptions.IgnoreCase)]
    private static partial Regex NeedsTokenRegex();

    [GeneratedRegex("已签到|已打卡|重复签到|已经签到|already", RegexOptions.IgnoreCase)]
    private static partial Regex AlreadyRegex();

    [GeneratedRegex("未启用|未开启|签到功能关闭|not enabled|disabled", RegexOptions.IgnoreCase)]
    private static partial Regex DisabledRegex();

    [GeneratedRegex("未登录|无权|登录已过期|请先登录|未授权|unauthorized|not logged in|invalid access token", RegexOptions.IgnoreCase)]
    private static partial Regex AuthRegex();

    [GeneratedRegex("未找到|不存在|not found|no such", RegexOptions.IgnoreCase)]
    private static partial Regex MissingEndpointRegex();

    private static readonly ConcurrentDictionary<string, string> PathCache = new(StringComparer.Ordinal);

    public static void ResetCaches() => PathCache.Clear();

    public static void Invalidate(string siteId) => PathCache.TryRemove(siteId, out _);

    public static IEnumerable<string> CandidateOrigins(Site site)
    {
        string alternate = site.Origin.StartsWith("https://", StringComparison.Ordinal)
            ? $"http://{Model.FormatHost(site.Host)}"
            : $"https://{Model.FormatHost(site.Host)}";
        yield return site.Origin;
        yield return alternate;
    }

    public static string FirstNonEmpty(params string[] values)
    {
        foreach (string value in values)
        {
            if (!string.IsNullOrEmpty(value))
 {
                return value;
            }
        }
        return "";
    }

    public static string ReadMessage(JsonNode? node)
    {
        if (node is not JsonObject map)
        {
            return "";
        }
        foreach (string key in MessageKeys)
        {
            bool found = map.TryGetPropertyValue(key, out JsonNode? value)
                && value is JsonValue typed
                && typed.TryGetValue<string>(out string? text)
                && !string.IsNullOrEmpty(text);
            if (found)
 {
                return (string)map[key]!;
            }
        }
        return "";
    }

    public static bool? ReadSuccess(JsonNode? node)
    {
        if (node is not JsonObject map)
        {
            return null;
        }
        if (map.TryGetPropertyValue("success", out JsonNode? flag)
            && flag is JsonValue flagValue
            && flagValue.TryGetValue<bool>(out bool parsedFlag))
        {
            return parsedFlag;
        }
        if (map.TryGetPropertyValue("code", out JsonNode? code) && code is JsonValue codeValue)
        {
            if (codeValue.TryGetValue<bool>(out bool boolCode))
 {
                return !boolCode;
            }
            if (codeValue.TryGetValue<int>(out int intCode))
 {
                return intCode == 0;
            }
            if (codeValue.TryGetValue<long>(out long longCode))
 {
                return longCode == 0;
            }
        }
        return null;
    }

    public static JsonObject? ReadData(JsonNode? node) => J.Obj(node, "data");

    public static string ReadCode(JsonNode? node)
    {
        if (node is not JsonObject map
            || !map.TryGetPropertyValue("code", out JsonNode? code)
            || code is not JsonValue value)
        {
            return "";
        }
        return value.TryGetValue<string>(out string? text) ? text : "";
    }

    private static string NormalizeDisplayType(string raw)
    {
        string text = Coerce.ToText(raw, "USD").Trim().ToUpperInvariant();
        return text.Length > 0 ? text[..Math.Min(12, text.Length)] : "USD";
    }


    private static int QuotaOf(JsonObject? data)
    {
        int raw = J.Int(data, "quota_per_unit", 0, 0, int.MaxValue);
        return Coerce.ToInt(raw == FlexibleIntConverter.Unparsable ? DefaultQuotaPerUnit : raw, DefaultQuotaPerUnit, 1, int.MaxValue);
    }

    public static SiteSnapshot? ReadMeta(JsonNode? node)
    {
        JsonObject? data = ReadData(node);
        if (data is null)
        {
            return null;
        }
        string version = J.Str(data, "version");
        return new SiteSnapshot
        {
            SystemName = J.Str(data, "system_name"),
            Version = version.Length > 0 ? version : null,
            CheckinEnabled = J.BoolOrNull(data, "checkin_enabled"),
            Turnstile = J.Bool(data, "turnstile_check"),
            QuotaPerUnit = J.Int(data, "quota_per_unit", DefaultQuotaPerUnit, 1, int.MaxValue),
            DisplayInCurrency = !data.ContainsKey("display_in_currency") || J.Bool(data, "display_in_currency"),
            DisplayType = NormalizeDisplayType(J.Str(data, "quota_display_type", "USD")),
            ExchangeRate = J.Dbl(data, "usd_exchange_rate", 1d, 0.000001d, 1_000_000d),
        };
    }

    public static SiteSnapshot? BuildMetaView(MetaEntry? info)
    {
        if (info is null)
 {
            return null;
        }
        string version = Coerce.ToText(info.Version);
        return new SiteSnapshot
        {
            Version = version.Length > 0 ? version : null,
            CheckinEnabled = info.CheckinEnabled,
            Turnstile = info.Turnstile,
            QuotaPerUnit = info.QuotaPerUnit > 0 ? info.QuotaPerUnit : DefaultQuotaPerUnit,
            DisplayInCurrency = info.DisplayInCurrency,
            DisplayType = NormalizeDisplayType(info.DisplayType),
            ExchangeRate = Coerce.ToFloat(info.ExchangeRate, 1d, 0.000001d, 1_000_000d),
        };
    }

    public static IReadOnlyList<string> OrderedPaths(string siteId)
    {
        if (!PathCache.TryGetValue(siteId, out string? learned))
        {
            return CheckinPaths;
        }
        List<string> result = new(CheckinPaths.Length) { learned };
        foreach (string path in CheckinPaths)
        {
            if (path != learned)
            {
                result.Add(path);
            }
        }
        return result;
    }

    public static void RememberPath(string siteId, string path) => PathCache[siteId] = path;
}
