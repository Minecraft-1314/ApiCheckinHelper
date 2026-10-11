using System.Text.Json.Serialization;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public sealed class SiteSnapshot
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string SystemName { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string? Version { get; set; }

    [JsonConverter(typeof(FlexibleBoolNullableConverter))]
    public bool? CheckinEnabled { get; set; }

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool Turnstile { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int QuotaPerUnit { get; set; } = 500000;

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool DisplayInCurrency { get; set; } = true;

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string DisplayType { get; set; } = "USD";

    [JsonConverter(typeof(FlexibleDoubleConverter))]
    public double ExchangeRate { get; set; } = 1d;

    public SiteSnapshot Clone() => new()
    {
        SystemName = SystemName,
        Version = Version,
        CheckinEnabled = CheckinEnabled,
        Turnstile = Turnstile,
        QuotaPerUnit = QuotaPerUnit,
        DisplayInCurrency = DisplayInCurrency,
        DisplayType = DisplayType,
        ExchangeRate = ExchangeRate,
    };
}

public sealed class Credential
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Type { get; set; } = "token";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Token { get; set; } = "";

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int UserId { get; set; }

    public Credential Clone() => new()
    {
        Type = Type,
        Token = Token,
        UserId = UserId
    };
}

public sealed class Site
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Id { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Name { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Host { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Origin { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Entry { get; set; } = "/";

    public List<string> Tags { get; set; } = new();

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Bookmark { get; set; } = "";

    public SiteSnapshot Snapshot { get; set; } = new();

    public Site Clone() => new()
    {
        Id = Id,
        Name = Name,
        Host = Host,
        Origin = Origin,
        Entry = Entry,
        Tags = new List<string>(Tags),
        Bookmark = Bookmark,
        Snapshot = Snapshot.Clone(),
    };
}

public sealed class MetaEntry
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string SystemName { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Version { get; set; } = "";

    [JsonConverter(typeof(FlexibleBoolNullableConverter))]
    public bool? CheckinEnabled { get; set; }

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool Turnstile { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int QuotaPerUnit { get; set; } = 500000;

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool DisplayInCurrency { get; set; } = true;

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string DisplayType { get; set; } = "USD";

    [JsonConverter(typeof(FlexibleDoubleConverter))]
    public double ExchangeRate { get; set; } = 1d;

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Account { get; set; } = "";

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int UserId { get; set; }

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long ProbedAt { get; set; }

    public MetaEntry Clone() => new()
    {
        SystemName = SystemName,
        Version = Version,
        CheckinEnabled = CheckinEnabled,
        Turnstile = Turnstile,
        QuotaPerUnit = QuotaPerUnit,
        DisplayInCurrency = DisplayInCurrency,
        DisplayType = DisplayType,
        ExchangeRate = ExchangeRate,
        Account = Account,
        UserId = UserId,
        ProbedAt = ProbedAt,
    };
}

public sealed class Settings
{
    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Concurrency { get; set; } = 3;

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int TimeoutMs { get; set; } = 15000;

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool Notify { get; set; } = true;

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool SkipManualOnly { get; set; } = true;

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Theme { get; set; } = "auto";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Language { get; set; } = "auto";

    public Settings Clone() => new()
    {
        Concurrency = Concurrency,
        TimeoutMs = TimeoutMs,
        Notify = Notify,
        SkipManualOnly = SkipManualOnly,
        Theme = Theme,
        Language = Language,
    };
}

public sealed class HistoryBucket
{
    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Total { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Success { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Already { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Failed { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Skipped { get; set; }

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long Quota { get; set; }

    [JsonConverter(typeof(FlexibleDoubleConverter))]
    public double QuotaUsd { get; set; }

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long At { get; set; }
}

public sealed class RunResult
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Id { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string State { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Message { get; set; } = "";

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long Quota { get; set; }

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Path { get; set; } = "";

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int HttpStatus { get; set; }

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long At { get; set; }

    public RunResult Clone() => new()
    {
        Id = Id,
        State = State,
        Message = Message,
        Quota = Quota,
        Path = Path,
        HttpStatus = HttpStatus,
        At = At,
    };
}

public sealed class VerifyEntry
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Id { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Name { get; set; } = "";

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Host { get; set; } = "";

    [JsonConverter(typeof(FlexibleBoolConverter))]
    public bool Ok { get; set; }

    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Message { get; set; } = "";
}

public sealed class VerifySummary
{
    [JsonConverter(typeof(FlexibleLongConverter))]
    public long At { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Total { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Ok { get; set; }

    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Failed { get; set; }

    public List<VerifyEntry> Items { get; set; } = new();

    public VerifySummary Clone() => new()
    {
        At = At,
        Total = Total,
        Ok = Ok,
        Failed = Failed,
        Items = Items.Select(item => new VerifyEntry {
            Id = item.Id,
            Name = item.Name,
            Host = item.Host,
            Ok = item.Ok,
            Message = item.Message,
        }).ToList(),
    };
}

public sealed class LastRun
{
    [JsonConverter(typeof(FlexibleStringConverter))]
    public string Day { get; set; } = "";

    [JsonConverter(typeof(FlexibleLongConverter))]
    public long At { get; set; }

    public LastRun Clone() => new() { Day = Day, At = At };
}

public sealed class AppState
{
    [JsonConverter(typeof(FlexibleIntConverter))]
    public int Version { get; set; } = 1;

    public Settings Settings { get; set; } = new();

    public List<Site> Sites { get; set; } = new();

    public Dictionary<string, Credential> Credentials { get; set; } = new(StringComparer.Ordinal);

    public Dictionary<string, MetaEntry> Meta { get; set; } = new(StringComparer.Ordinal);

    public Dictionary<string, RunResult?> Results { get; set; } = new(StringComparer.Ordinal);

    public LastRun LastRun { get; set; } = new();

    public VerifySummary Verify { get; set; } = new();

    public Dictionary<string, HistoryBucket> History { get; set; } = new(StringComparer.Ordinal);

    public AppState Clone()
    {
        return new AppState {
            Version = Version,
            Settings = Settings.Clone(),
            Sites = Sites.Select(site => site.Clone()).ToList(),
            Credentials = Credentials.ToDictionary(pair => pair.Key, pair => pair.Value.Clone(), StringComparer.Ordinal),
            Meta = Meta.ToDictionary(pair => pair.Key, pair => pair.Value.Clone(), StringComparer.Ordinal),
            Results = Results.ToDictionary(pair => pair.Key, pair => pair.Value?.Clone(), StringComparer.Ordinal),
            LastRun = LastRun.Clone(),
            Verify = Verify.Clone(),
            History = History.ToDictionary(pair => pair.Key, pair => new HistoryBucket {
                Total = pair.Value.Total,
                Success = pair.Value.Success,
                Already = pair.Value.Already,
                Failed = pair.Value.Failed,
                Skipped = pair.Value.Skipped,
                Quota = pair.Value.Quota,
                QuotaUsd = pair.Value.QuotaUsd,
                At = pair.Value.At,
            }, StringComparer.Ordinal),
        };
    }
}
