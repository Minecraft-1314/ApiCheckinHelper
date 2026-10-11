using System.Collections.Concurrent;
using System.Diagnostics;

namespace ApiCheckinHelper.Core;

public sealed class PlanEntry
{
    public required Site Site { get; init; }

    public required Credential Credential { get; init; }

    public required MetaEntry Meta { get; init; }
}

public sealed class CheckinSummary
{
    public int Total { get; init; }

    public long ElapsedMs { get; init; }

    public Dictionary<string, RunResult> Results { get; init; } = new(StringComparer.Ordinal);
}

public static class Runner
{
    public const int MaxWorkers = 8;

    public const int VerifyExtraWorkers = 2;

    public static string? SkipReason(PlanEntry entry, Settings settings)
    {
        if (!Model.IsConfigured(entry.Credential))
        {
            return "未配置访问令牌";
        }
        if (settings.SkipManualOnly && entry.Meta.Turnstile)
        {
            return "该站点需人机验证，仅能手动完成";
        }
        return null;
    }

    public static (List<PlanEntry> Plan, Settings Settings) BuildPlan(string? onlySiteId = null)
    {
        AppState state = ConfigStore.Load();
        Settings settings = Model.NormalizeSettings(state.Settings);
        List<PlanEntry> plan = new();
        foreach (Site site in state.Sites)
        {
            if (onlySiteId is not null && site.Id != onlySiteId)
            {
                continue;
            }
            state.Credentials.TryGetValue(site.Id, out Credential? credential);
            state.Meta.TryGetValue(site.Id, out MetaEntry? meta);
            plan.Add(new PlanEntry {
                Site = site.Clone(),
                Credential = (credential ?? new Credential()).Clone(),
                Meta = ConfigStore.Copy(meta ?? new MetaEntry()),
            });
        }
        return (plan, settings);
    }

    public static MetaEntry MetaPatch(SiteSnapshot? meta)
    {
        if (meta is null)
        {
            return new MetaEntry();
        }
        return new MetaEntry {
            SystemName = Coerce.ToText(meta.SystemName),
            Version = Coerce.ToText(meta.Version),
            CheckinEnabled = meta.CheckinEnabled,
            Turnstile = meta.Turnstile,
            QuotaPerUnit = meta.QuotaPerUnit,
            DisplayInCurrency = meta.DisplayInCurrency,
            DisplayType = meta.DisplayType,
            ExchangeRate = meta.ExchangeRate,
        };
    }

    private static RunResult Skipped(PlanEntry entry, string reason)
    {
        return new RunResult {
            Id = entry.Site.Id,
            State = "skipped",
            Message = reason,
        };
    }

    public sealed class Collected
    {
        public ConcurrentDictionary<string, MetaEntry> Metas { get; } = new(StringComparer.Ordinal);
    }

    private static async Task<RunResult?> RunOneAsync(
        PlanEntry entry,
        Settings settings,
        Collected collected,
        CancellationToken cancellation)
    {
        string? reason = SkipReason(entry, settings);
        if (reason is not null)
        {
            return Skipped(entry, reason);
        }
        CheckinOutcome outcome;
        try
        {
            outcome = await Checkin.RunAsync(entry.Site, entry.Credential, settings.TimeoutMs, entry.Meta).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception error)
        {
            outcome = new CheckinOutcome {
                State = "failed",
                Message = $"执行异常：{error.GetType().Name}: {error.Message}",
            };
        }
        if (outcome.Meta is not null)
        {
            MetaEntry patch = MetaPatch(outcome.Meta);
            patch.Account = outcome.Account.Length > 0 ? outcome.Account : entry.Meta.Account;
            patch.UserId = outcome.UserId > 0 ? outcome.UserId : entry.Meta.UserId;
            collected.Metas[entry.Site.Id] = patch;
        }
        return new RunResult {
            Id = entry.Site.Id,
            State = outcome.State,
            Message = outcome.Message,
            Quota = Coerce.ToLong(outcome.Quota, 0, 0, long.MaxValue),
            Path = outcome.Path,
            HttpStatus = outcome.HttpStatus,
        };
    }

    private static void Persist(Collected collected)
    {
        using IDisposable batch = ConfigStore.Batch();
        if (!collected.Metas.IsEmpty)
        {
            ConfigStore.SetMetaBulk(collected.Metas);
        }
    }

    public static async Task<CheckinSummary> RunCheckinAsync(
        string? onlySiteId,
        IProgress<RunResult>? onProgress = null,
        CancellationToken cancellation = default)
    {
        cancellation.ThrowIfCancellationRequested();
        Stopwatch watch = Stopwatch.StartNew();
        long startedAt = ConfigStore.NowMs();
        (List<PlanEntry> plan, Settings settings) = BuildPlan(onlySiteId);
        if (plan.Count == 0)
        {
            return new CheckinSummary { Total = 0 };
        }
        int limit = Math.Clamp(settings.Concurrency, 1, MaxWorkers);
        Collected collected = new();
        ConcurrentDictionary<string, RunResult> results = new(StringComparer.Ordinal);
        using SemaphoreSlim gate = new(limit, limit);
        Task[] tasks = plan.Select(async entry =>
        {
            if (cancellation.IsCancellationRequested)
            {
                return;
            }
            await gate.WaitAsync(cancellation).ConfigureAwait(false);
            RunResult? item;
            try
            {
                item = await RunOneAsync(entry, settings, collected, cancellation).ConfigureAwait(false);
            }
            finally
            {
                gate.Release();
            }
            if (item is null)
            {
                return;
            }
            results[item.Id] = item;
            onProgress?.Report(item);
        }).ToArray();
        await Task.WhenAll(tasks).ConfigureAwait(false);
        cancellation.ThrowIfCancellationRequested();
        Persist(collected);
        List<RunResult> ordered = plan
            .Select(entry => results.GetValueOrDefault(entry.Site.Id))
            .OfType<RunResult>()
            .ToList();
        ConfigStore.RecordRun(ordered, startedAt);
        watch.Stop();
        return new CheckinSummary {
            Total = plan.Count,
            ElapsedMs = watch.ElapsedMilliseconds,
            Results = results.ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.Ordinal),
        };
    }

    public static async Task<VerifySummary> RunVerifyAsync(
        IProgress<VerifyEntry>? onProgress = null,
        CancellationToken cancellation = default)
    {
        cancellation.ThrowIfCancellationRequested();
        (List<PlanEntry> plan, Settings settings) = BuildPlan();
        List<PlanEntry> targets = plan.Where(entry => Model.IsConfigured(entry.Credential)).ToList();
        if (targets.Count == 0)
        {
            return new VerifySummary();
        }
        int limit = Math.Clamp(settings.Concurrency + VerifyExtraWorkers, 1, MaxWorkers);
        List<VerifyEntry> items = new();
        ConcurrentDictionary<string, MetaEntry> metas = new(StringComparer.Ordinal);
        int okCount = 0;
        int failedCount = 0;
        using SemaphoreSlim gate = new(limit, limit);
        Task[] tasks = targets.Select(async entry =>
        {
            if (cancellation.IsCancellationRequested)
            {
                return;
            }
            await gate.WaitAsync(cancellation).ConfigureAwait(false);
            VerifyOutcome outcome;
            try
            {
                outcome = await Verify.RunAsync(entry.Site, entry.Credential, settings.TimeoutMs, entry.Meta).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (cancellation.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception error)
            {
                outcome = new VerifyOutcome {
                    Ok = false,
                    Message = $"校验异常：{error.GetType().Name}: {error.Message}",
                };
            }
            finally
            {
                gate.Release();
            }
            if (outcome.Ok)
            {
                Interlocked.Increment(ref okCount);
                metas[entry.Site.Id] = new MetaEntry {
                    Account = outcome.Account.Length > 0 ? outcome.Account : entry.Meta.Account,
                    UserId = outcome.UserId > 0 ? outcome.UserId : entry.Meta.UserId,
                };
            }
            else
            {
                Interlocked.Increment(ref failedCount);
            }
            VerifyEntry record = new() {
                Id = entry.Site.Id,
                Name = entry.Site.Name,
                Host = entry.Site.Host,
                Ok = outcome.Ok,
                Message = outcome.Message,
            };
            lock (items)
            {
                items.Add(record);
            }
            onProgress?.Report(record);
        }).ToArray();
        await Task.WhenAll(tasks).ConfigureAwait(false);
        cancellation.ThrowIfCancellationRequested();
        VerifySummary summary = new() {
            Total = targets.Count,
            Ok = okCount,
            Failed = failedCount,
            Items = items,
        };
        using (IDisposable batch = ConfigStore.Batch())
        {
            if (!metas.IsEmpty)
            {
                ConfigStore.SetMetaBulk(metas);
            }
            ConfigStore.SetVerify(summary);
        }
        return summary;
    }

    public static async Task<CheckinOutcome> ProbeAsync(string siteId, CancellationToken cancellation = default)
    {
        Site? site = ConfigStore.FindSite(siteId);
        if (site is null)
        {
            return new CheckinOutcome { State = "failed", Message = "站点不存在" };
        }
        Settings settings = ConfigStore.Settings();
        MetaEntry meta = ConfigStore.MetaFor(siteId);
        CheckinOutcome outcome;
        try
        {
            outcome = await Checkin.ProbeAsync(site, settings.TimeoutMs, meta, cancellation).ConfigureAwait(false);
        }
        catch (NetError error)
        {
            outcome = new CheckinOutcome { State = "offline", Message = error.Message };
        }
        if (outcome.Meta is not null)
        {
            ConfigStore.SetMeta(siteId, MetaPatch(outcome.Meta));
        }
        return outcome;
    }
}
