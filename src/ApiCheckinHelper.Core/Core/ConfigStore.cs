using System.Globalization;
using System.Text;
using System.Text.Json;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public sealed class ConfigError(string message) : Exception(message);

public static class ConfigStore
{
    private sealed class BatchScope : IDisposable
    {
        public BatchScope()
        {
            lock (Gate)
            {
                _batchDepth++;
            }
        }

        public void Dispose()
        {
            lock (Gate)
            {
                _batchDepth = Math.Max(0, _batchDepth - 1);
                if (_batchDepth == 0)
                {
                    WriteNow(false);
                }
            }
        }
    }

    private static readonly object Gate = new();
    private static AppState? _state;
    private static long _revision;
    private static bool _dirty;
    private static int _batchDepth;
    private static string _lastError = "";

    public static string DataDirectory => Paths.DataDirectory;

    public static string ConfigPath => Paths.ConfigPath;

    public static string LastError
    {
        get
        {
            lock (Gate)
            {
                return _lastError;
            }
        }
    }

    public static long Revision
    {
        get
        {
            lock (Gate)
            {
                return _revision;
            }
        }
    }

    public static string TodayKey() => DateTime.Now.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);

    public static long NowMs() => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();

    public static void Bind()
    {
        Paths.Bind();
        AppLog.Bind();
    }

    public static AppState Load(bool refresh = false)
    {
        lock (Gate)
        {
            AppState state = LoadLocked(refresh);
            return state.Clone();
        }
    }

    public static AppState Normalize(AppState source)
    {
        source ??= new AppState();
        AppState normalized = new()
        {
            Version = Model.StateVersion,
            Settings = Model.NormalizeSettings(source.Settings),
            Sites = Model.NormalizeSiteList(source.Sites),
            LastRun = new LastRun
            {
                Day = Coerce.ToText(source.LastRun?.Day),
                At = Coerce.ToLong(source.LastRun?.At, 0, 0, long.MaxValue),
            },
            Verify = Model.NormalizeVerify(source.Verify),
        };
        Dictionary<string, Credential> credentials = source.Credentials ?? new(StringComparer.Ordinal);
        Dictionary<string, MetaEntry> meta = source.Meta ?? new(StringComparer.Ordinal);
        Dictionary<string, RunResult?> results = source.Results ?? new(StringComparer.Ordinal);
        Dictionary<string, HistoryBucket> history = source.History ?? new(StringComparer.Ordinal);
        foreach (Site site in normalized.Sites)
        {
            credentials.TryGetValue(site.Id, out Credential? credential);
            normalized.Credentials[site.Id] = Model.NormalizeCredential(credential);
            meta.TryGetValue(site.Id, out MetaEntry? info);
            normalized.Meta[site.Id] = info is null ? new MetaEntry() : Copy(info);
            results.TryGetValue(site.Id, out RunResult? result);
            normalized.Results[site.Id] = Model.NormalizeResult(result);
        }
        foreach (KeyValuePair<string, HistoryBucket> pair in history)
        {
            normalized.History[pair.Key] = Model.NormalizeHistoryBucket(pair.Value);
        }
        normalized.History = Model.PruneHistory(normalized.History);
        return normalized;
    }

    public static bool Flush()
    {
        lock (Gate)
        {
            return !_dirty || WriteNow(true);
        }
    }

    public static IDisposable Batch() => new BatchScope();

    public static AppState Update(Action<AppState> mutator)
    {
        ArgumentNullException.ThrowIfNull(mutator);
        lock (Gate)
        {
            AppState state = LoadLocked(false);
            mutator(state);
            _revision++;
            _dirty = true;
            if (_batchDepth == 0)
            {
                WriteNow(false);
            }
            return state.Clone();
        }
    }

    public static List<Site> Sites()
    {
        lock (Gate)
        {
            return LoadLocked(false).Sites.Select(site => site.Clone()).ToList();
        }
    }

    public static Site? FindSite(string siteId)
    {
        lock (Gate)
        {
            return LoadLocked(false).Sites.FirstOrDefault(site => site.Id == siteId)?.Clone();
        }
    }

    public static string StripWww(string host)
    {
        string normalized = Coerce.ToText(host).Trim().ToLowerInvariant();
        return normalized.StartsWith("www.", StringComparison.Ordinal) ? normalized[4..] : normalized;
    }

    public static Site? FindByHost(string? host)
    {
        string key = StripWww(Coerce.ToText(host));
        lock (Gate)
        {
            return LoadLocked(false)
                .Sites
                .FirstOrDefault(site => StripWww(site.Host) == key)
                ?.Clone();
        }
    }

    public static Credential CredentialsFor(string siteId)
    {
        lock (Gate)
        {
            return LoadLocked(false).Credentials.TryGetValue(siteId, out Credential? credential)
                ? credential.Clone()
                : Model.NormalizeCredential(null);
        }
    }

    public static Settings Settings()
    {
        lock (Gate)
        {
            return LoadLocked(false).Settings.Clone();
        }
    }

    public static MetaEntry MetaFor(string siteId)
    {
        lock (Gate)
        {
            return LoadLocked(false).Meta.TryGetValue(siteId, out MetaEntry? meta)
                ? Copy(meta)
                : new MetaEntry();
        }
    }

    public static MetaEntry Copy(MetaEntry source)
    {
        ArgumentNullException.ThrowIfNull(source);
        return source.Clone();
    }

    public static RunResult? ResultFor(string siteId)
    {
        lock (Gate)
        {
            return LoadLocked(false).Results.TryGetValue(siteId, out RunResult? result)
                ? Model.NormalizeResult(result)
                : null;
        }
    }

    public static void PatchSettings(Settings patch)
    {
        ArgumentNullException.ThrowIfNull(patch);
        Settings normalized = Model.NormalizeSettings(patch);
        Update(state => state.Settings = normalized);
    }

    public static void SetCredential(string siteId, Credential patch)
    {
        ArgumentNullException.ThrowIfNull(patch);
        Update(state =>
        {
            if (!state.Credentials.TryGetValue(siteId, out Credential? current))
            {
                return;
            }
            Credential next = current.Clone();
            next.Type = "token";
            next.Token = patch.Token;
            next.UserId = patch.UserId;
            state.Credentials[siteId] = Model.NormalizeCredential(next);
        });
    }

    public static void SaveCredential(string siteId, Credential patch)
    {
        ArgumentNullException.ThrowIfNull(patch);
        Update(state =>
        {
            if (!state.Credentials.TryGetValue(siteId, out Credential? current))
            {
                return;
            }
            Credential next = current.Clone();
            next.Type = "token";
            next.Token = patch.Token;
            next.UserId = patch.UserId;
            state.Credentials[siteId] = Model.NormalizeCredential(next);
        });
    }

    public static void SetCredentials(IReadOnlyDictionary<string, Credential> patches)
    {
        ArgumentNullException.ThrowIfNull(patches);
        Update(state =>
        {
            foreach (KeyValuePair<string, Credential> pair in patches)
            {
                if (!state.Credentials.TryGetValue(pair.Key, out Credential? current))
                {
                    continue;
                }
                Credential patch = pair.Value;
                Credential next = current.Clone();
                next.Type = "token";
                next.Token = patch.Token;
                if (patch.UserId > 0)
                {
                    next.UserId = patch.UserId;
                }
                state.Credentials[pair.Key] = Model.NormalizeCredential(next);
            }
        });
    }

    public static void ClearCredential(string siteId)
    {
        Update(state =>
        {
            state.Credentials[siteId] = Model.NormalizeCredential(null);
        });
    }

    public static void SetUserId(string siteId, int userId)
    {
        Update(state =>
        {
            if (state.Credentials.TryGetValue(siteId, out Credential? current))
            {
                current.UserId = Coerce.ToInt(userId, 0, 0, 1_000_000_000);
            }
        });
    }

    public static void SetMeta(string siteId, MetaEntry patch)
    {
        ArgumentNullException.ThrowIfNull(patch);
        long stamp = NowMs();
        Update(state =>
        {
            MetaEntry next = state.Meta.TryGetValue(siteId, out MetaEntry? current)
                ? Copy(current)
                : new MetaEntry();
            ApplyMeta(next, patch);
            next.ProbedAt = stamp;
            state.Meta[siteId] = next;
        });
    }

    public static void SetMetaBulk(IReadOnlyDictionary<string, MetaEntry> patches)
    {
        ArgumentNullException.ThrowIfNull(patches);
        long stamp = NowMs();
        Update(state =>
        {
            foreach (KeyValuePair<string, MetaEntry> pair in patches)
            {
                if (!state.Meta.TryGetValue(pair.Key, out MetaEntry? current))
                {
                    continue;
                }
                MetaEntry next = Copy(current);
                ApplyMeta(next, pair.Value);
                next.ProbedAt = stamp;
                state.Meta[pair.Key] = next;
            }
        });
    }

    public static void SetVerify(VerifySummary summary)
    {
        ArgumentNullException.ThrowIfNull(summary);
        VerifySummary normalized = Model.NormalizeVerify(summary);
        normalized.At = NowMs();
        Update(state => state.Verify = normalized);
    }

    public static Site AddSite(string host, string name = "")
    {
        Site? normalized = Model.NormalizeSite(new Site { Host = host, Name = name });
        if (normalized is null)
        {
            throw new ConfigError("站点地址不合法，请填写形如 api.example.com 的域名");
        }
        lock (Gate)
        {
            AppState state = LoadLocked(false);
            if (state.Sites.Any(site => site.Host == normalized.Host))
            {
                throw new ConfigError("这个域名已经在列表里了：" + normalized.Host);
            }
            string baseId = normalized.Id;
            HashSet<string> ids = state.Sites.Select(site => site.Id).ToHashSet(StringComparer.Ordinal);
            int suffix = 2;
            while (ids.Contains(normalized.Id))
            {
                normalized.Id = $"{baseId}-{suffix}";
                suffix++;
            }
            Site added = normalized;
            Update(draft =>
            {
                draft.Sites.Add(added.Clone());
                Attach(draft, added);
            });
            return added.Clone();
        }
    }

    public static Site UpdateSite(string siteId, string? host, string? name)
    {
        lock (Gate)
        {
            Site? current = LoadLocked(false).Sites.FirstOrDefault(site => site.Id == siteId);
            if (current is null)
            {
                throw new ConfigError("站点不存在");
            }
            Site draft = current.Clone();
            if (host is not null)
            {
                draft.Host = host;
            }
            if (name is not null)
            {
                draft.Name = name;
            }
            Site? normalized = Model.NormalizeSite(draft);
            if (normalized is null)
            {
                throw new ConfigError("站点地址不合法");
            }
            normalized.Id = siteId;
            if (LoadLocked(false).Sites.Any(site => site.Host == normalized.Host && site.Id != siteId))
            {
                throw new ConfigError("这个域名已被其他站点占用：" + normalized.Host);
            }
            Site next = normalized;
            Update(state =>
            {
                int index = state.Sites.FindIndex(site => site.Id == siteId);
                if (index >= 0)
                {
                    state.Sites[index] = next.Clone();
                }
                Attach(state, next);
            });
            return next.Clone();
        }
    }

    public static void RemoveSite(string siteId)
    {
        lock (Gate)
        {
            if (LoadLocked(false).Sites.All(site => site.Id != siteId))
            {
                throw new ConfigError("站点不存在");
            }
            Update(state =>
            {
                state.Sites.RemoveAll(site => site.Id == siteId);
                state.Credentials.Remove(siteId);
                state.Meta.Remove(siteId);
                state.Results.Remove(siteId);
            });
        }
    }

    public static void ClearCredentials()
    {
        Update(state =>
        {
            foreach (string siteId in state.Credentials.Keys)
            {
                state.Credentials[siteId] = Model.NormalizeCredential(null);
            }
        });
    }

    public static AppState ResetAll()
    {
        lock (Gate)
        {
            try
            {
                if (File.Exists(Paths.ConfigPath))
                {
                    File.Delete(Paths.ConfigPath);
                }
            }
            catch (IOException error)
            {
                _lastError = "现有配置无法删除，已回退到空配置";
                AppLog.Error("现有配置无法删除", error.Message);
            }
            catch (UnauthorizedAccessException error)
            {
                _lastError = "现有配置无法删除，已回退到空配置";
                AppLog.Error("现有配置无法删除", error.Message);
            }
            _state = null;
            _dirty = false;
            AppState state = LoadLocked(true);
            if (_batchDepth == 0)
            {
                WriteNow(false);
            }
            return state.Clone();
        }
    }

    public static void RecordRun(IReadOnlyList<RunResult> results, long startedAt)
    {
        ArgumentNullException.ThrowIfNull(results);
        Update(state =>
        {
            string day = TodayKey();
            HistoryBucket bucket = state.History.TryGetValue(day, out HistoryBucket? current)
                ? CopyBucket(current)
                : new HistoryBucket();
            Dictionary<string, int> units = state.Sites.ToDictionary(
                site => site.Id,
                site => site.Snapshot.QuotaPerUnit > 0 ? site.Snapshot.QuotaPerUnit : Model.DefaultQuotaPerUnit,
                StringComparer.Ordinal);
            foreach (RunResult result in results)
            {
                if (state.Results.ContainsKey(result.Id))
                {
                    RunResult stored = result.Clone();
                    stored.Id = result.Id;
                    stored.At = NowMs();
                    state.Results[result.Id] = stored;
                }
                bucket.Total++;
                if (result.State == "success")
                {
                    bucket.Success++;
                    long quota = Coerce.ToLong(result.Quota, 0, 0, long.MaxValue);
                    bucket.Quota += quota;
                    int unit = units.GetValueOrDefault(result.Id, Model.DefaultQuotaPerUnit);
                    if (unit <= 0)
                    {
                        unit = Model.DefaultQuotaPerUnit;
                    }
                    bucket.QuotaUsd += quota / (double)unit;
                }
                else if (result.State == "already")
                {
                    bucket.Already++;
                }
                else if (result.State == "skipped")
                {
                    bucket.Skipped++;
                }
                else
                {
                    bucket.Failed++;
                }
            }
            bucket.At = NowMs();
            state.History[day] = bucket;
            state.History = Model.PruneHistory(state.History);
            state.LastRun = new LastRun
            {
                Day = day,
                At = Coerce.ToLong(startedAt, 0, 0, long.MaxValue),
            };
        });
    }

    public static HistoryBucket HistoryFor(string? day = null)
    {
        string key = string.IsNullOrEmpty(day) ? TodayKey() : day;
        lock (Gate)
        {
            return LoadLocked(false).History.TryGetValue(key, out HistoryBucket? bucket)
                ? Model.NormalizeHistoryBucket(bucket)
                : new HistoryBucket();
        }
    }

    public static LastRun LastRun()
    {
        lock (Gate)
        {
            LastRun lastRun = LoadLocked(false).LastRun;
            return new LastRun { Day = lastRun.Day, At = lastRun.At };
        }
    }

    public static VerifySummary VerifySummary()
    {
        lock (Gate)
        {
            return Model.NormalizeVerify(LoadLocked(false).Verify);
        }
    }

    private static AppState LoadLocked(bool refresh)
    {
        if (_state is not null && !refresh)
        {
            return _state;
        }
        Paths.Bind();
        _lastError = "";
        _state = BuildInitial();
        return _state;
    }

    private static void Quarantine(string path, Exception error, string userMessage)
    {
        long stamp = NowMs();
        string target = Path.Combine(Paths.DataDirectory, $"config.corrupt-{stamp}.json");
        int suffix = 1;
        while (File.Exists(target))
        {
            target = Path.Combine(Paths.DataDirectory, $"config.corrupt-{stamp}-{suffix}.json");
            suffix++;
        }
        try
        {
            File.Move(path, target, true);
            _lastError = userMessage;
            AppLog.Error("配置文件损坏，已备份为 " + Path.GetFileName(target), error.Message);
        }
        catch (IOException backupError)
        {
            _lastError = "配置文件无法隔离，已暂时回退到空配置";
            AppLog.Error("配置文件损坏且无法备份", backupError.ToString());
        }
        catch (UnauthorizedAccessException backupError)
        {
            _lastError = "配置文件无法隔离，已暂时回退到空配置";
            AppLog.Error("配置文件损坏且无法备份", backupError.ToString());
        }
    }

    private static AppState BuildInitial()
    {
        Paths.EnsureDataDirectory();
        if (!File.Exists(Paths.ConfigPath))
        {
            return Normalize(new AppState());
        }
        try
        {
            FileInfo info = new(Paths.ConfigPath);
            if (info.Length > Model.MaxConfigBytes)
            {
                Quarantine(Paths.ConfigPath, new InvalidDataException("配置文件超过大小上限"), "配置文件超过大小上限，已备份并回退到空配置");
                return Normalize(new AppState());
            }
            string json = File.ReadAllText(Paths.ConfigPath, Encoding.UTF8);
            ConfigSchema.Validate(json);
            AppState? loaded = JsonSerializer.Deserialize<AppState>(
                json,
                Flexible.Options);
            if (loaded is null)
            {
                throw new InvalidDataException("配置内容为空");
            }
            return Normalize(loaded);
        }
        catch (Exception error) when (
            error is InvalidDataException
                or JsonException
                or NotSupportedException
                or ArgumentException
                or InvalidOperationException)
        {
            Quarantine(Paths.ConfigPath, error, "配置文件不符合当前格式，已备份并回退到空配置");
            AppLog.Error("读取配置失败，回退到初始状态", error.ToString());
            return Normalize(new AppState());
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            _lastError = "配置文件暂时无法读取，已回退到空配置";
            AppLog.Error("配置文件无法读取，回退到初始状态", error.ToString());
            return Normalize(new AppState());
        }
        catch (Exception error)
        {
            _lastError = "配置读取失败，已回退到空配置";
            AppLog.Error("读取配置失败，回退到初始状态", error.ToString());
            return Normalize(new AppState());
        }
    }

    private static bool WriteNow(bool durable)
    {
        if (_state is null)
        {
            _dirty = false;
            return true;
        }
        string tempPath = Paths.ConfigPath + "." + Environment.ProcessId + ".tmp";
        try
        {
            Paths.EnsureDataDirectory();
            byte[] bytes = JsonSerializer.SerializeToUtf8Bytes(_state, Flexible.Options);
            using (FileStream stream = new(tempPath, FileMode.Create, FileAccess.Write, FileShare.None))
            {
                stream.Write(bytes);
                stream.Flush(durable);
            }
            File.Move(tempPath, Paths.ConfigPath, true);
            _dirty = false;
            _lastError = "";
            return true;
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or NotSupportedException or JsonException or ArgumentException)
        {
            _lastError = "配置写入失败，本次改动仅保留在内存中";
            AppLog.Error("写入配置失败，本次改动仅保留在内存中", error.ToString());
            try
            {
                if (File.Exists(tempPath))
                {
                    File.Delete(tempPath);
                }
            }
            catch (Exception cleanupError)
            {
                AppLog.Error("临时配置文件清理失败", cleanupError.Message);
            }
            return false;
        }
    }

    private static void Attach(AppState state, Site site)
    {
        state.Credentials.TryAdd(site.Id, Model.NormalizeCredential(null));
        state.Meta.TryAdd(site.Id, new MetaEntry());
        state.Results.TryAdd(site.Id, null);
    }

    private static void PruneOrphans(AppState state)
    {
        HashSet<string> keep = state.Sites.Select(site => site.Id).ToHashSet(StringComparer.Ordinal);
        foreach (string key in state.Credentials.Keys.Where(key => !keep.Contains(key)).ToList())
        {
            state.Credentials.Remove(key);
        }
        foreach (string key in state.Meta.Keys.Where(key => !keep.Contains(key)).ToList())
        {
            state.Meta.Remove(key);
        }
        foreach (string key in state.Results.Keys.Where(key => !keep.Contains(key)).ToList())
        {
            state.Results.Remove(key);
        }
    }

    private static void ApplyMeta(MetaEntry target, MetaEntry patch)
    {
        target.SystemName = patch.SystemName;
        target.Version = patch.Version;
        target.CheckinEnabled = patch.CheckinEnabled;
        target.Turnstile = patch.Turnstile;
        target.QuotaPerUnit = patch.QuotaPerUnit;
        target.DisplayInCurrency = patch.DisplayInCurrency;
        target.DisplayType = patch.DisplayType;
        target.ExchangeRate = patch.ExchangeRate;
        if (patch.Account.Length > 0)
        {
            target.Account = patch.Account;
        }
        if (patch.UserId > 0)
        {
            target.UserId = patch.UserId;
        }
    }

    private static HistoryBucket CopyBucket(HistoryBucket source) => new()
    {
        Total = source.Total,
        Success = source.Success,
        Already = source.Already,
        Failed = source.Failed,
        Skipped = source.Skipped,
        Quota = source.Quota,
        QuotaUsd = source.QuotaUsd,
        At = source.At,
    };
}
