namespace ApiCheckinHelper.Core;

public sealed class CheckinOutcome
{
    public string State { get; set; } = "failed";

    public int HttpStatus { get; set; }

    public string Message { get; set; } = "";

    public long Quota { get; set; }

    public string CheckinDate { get; set; } = "";

    public string Path { get; set; } = "";

    public SiteSnapshot? Meta { get; set; }

    public int UserId { get; set; }

    public string Account { get; set; } = "";
}

public static class Checkin
{
    public const string UserHeader = "New-Api-User";

    public const double PathBudgetFactor = 2d;

    public static TimeSpan Budget(int timeoutMs, double factor)
    {
        int ms = Coerce.ToInt(timeoutMs, Net.DefaultTimeoutMs, 500, 600000);
        return TimeSpan.FromSeconds(Math.Max(3d, ms / 1000d) * factor);
    }

    public static async Task<CheckinOutcome> ProbeAsync(
        Site site,
        int timeoutMs,
        MetaEntry? cachedMeta = null,
        CancellationToken deadline = default)
    {
        StatusResult status = await Protocol.FetchStatusAsync(site, timeoutMs, cachedMeta, deadline).ConfigureAwait(false);
        return new CheckinOutcome {
            State = status.State,
            Message = status.Message,
            Meta = status.Meta,
            HttpStatus = status.HttpStatus,
        };
    }

    private static void ResolveUser(
        Dictionary<string, string> headers,
        CheckinOutcome outcome,
        Credential credential,
        MetaEntry? cachedMeta)
    {
        if (outcome.UserId > 0) {
            headers[UserHeader] = outcome.UserId.ToString(System.Globalization.CultureInfo.InvariantCulture);
            return;
        }
        int manual = Coerce.ToInt(credential.UserId, 0, 0, 1_000_000_000);
        int known = Coerce.ToInt(cachedMeta?.UserId, 0, 0, 1_000_000_000);
        int candidate = manual > 0 ? manual : known;
        if (candidate > 0) {
            headers[UserHeader] = candidate.ToString(System.Globalization.CultureInfo.InvariantCulture);
            outcome.UserId = candidate;
        }
    }

    private static CheckinVerdict Tag(CheckinVerdict verdict, string path, int status)
    {
        verdict.Path = path;
        verdict.HttpStatus = status;
        return verdict;
    }

    private static async Task<CheckinVerdict> AttemptPathsAsync(
        Site site,
        IReadOnlyDictionary<string, string> headers,
        int timeoutMs,
        CancellationToken deadline)
    {
        CheckinVerdict? skipped = null;
        foreach (string path in Protocol.OrderedPaths(site.Id)) {
            if (deadline.CanBeCanceled && deadline.IsCancellationRequested) {
                return new CheckinVerdict {
                    State = "failed",
                    Message = "已用完本次签到的时间预算",
                    Path = path,
                };
            }
            NetResponse response;
            try {
                response = await Net.PostAsync($"{site.Origin}{path}", timeoutMs, headers, null, deadline)
                    .ConfigureAwait(false);
            } catch (NetError error) {
                return new CheckinVerdict {
                    State = NetErrors.OfflineState(error),
                    Message = error.Message,
                    Path = path,
                };
            }
            int status = response.Status;
            if (status is 404 or 405 or 501) {
                continue;
            }
            CheckinVerdict verdict = Tag(Protocol.Interpret(status, response), path, status);
            if (verdict.State is "success" or "already") {
                Protocol.RememberPath(site.Id, path);
                return verdict;
            }
            if (verdict.State != "skip") {
                return verdict;
            }
            skipped ??= verdict;
        }
        if (skipped is not null) {
            return skipped;
        }
        return new CheckinVerdict {
            State = "unsupported",
            Message = "未找到可用的签到接口",
        };
    }

    private static async Task<CheckinVerdict> AttemptWithRetryAsync(
        Site site,
        Credential credential,
        Dictionary<string, string> headers,
        CheckinOutcome outcome,
        MetaEntry? cachedMeta,
        int timeoutMs,
        CancellationToken deadline)
    {
        using CancellationTokenSource pathBudget = new(Budget(timeoutMs, PathBudgetFactor));
        using CancellationTokenSource pathLinked = CancellationTokenSource.CreateLinkedTokenSource(pathBudget.Token, deadline);
        CheckinVerdict verdict = await AttemptPathsAsync(site, headers, timeoutMs, pathLinked.Token).ConfigureAwait(false);
        bool retryable = verdict.State == "unauthorized" && !headers.ContainsKey(UserHeader);
        if (!retryable) {
            return verdict;
        }
        int known = Coerce.ToInt(cachedMeta?.UserId, 0, 0, 1_000_000_000);
        if (known <= 0) {
            return verdict;
        }
        headers[UserHeader] = known.ToString(System.Globalization.CultureInfo.InvariantCulture);
        outcome.UserId = known;
        using CancellationTokenSource retryBudget = new(Budget(timeoutMs, PathBudgetFactor));
        using CancellationTokenSource retryLinked = CancellationTokenSource.CreateLinkedTokenSource(retryBudget.Token, deadline);
        CheckinVerdict retry = await AttemptPathsAsync(site, headers, timeoutMs, retryLinked.Token).ConfigureAwait(false);
        return retry.State is "unauthorized" or "failed" ? verdict : retry;
    }

    public static async Task<CheckinOutcome> RunAsync(
        Site site,
        Credential credential,
        int timeoutMs,
        MetaEntry? cachedMeta = null,
        CancellationToken? outerDeadline = null)
    {
        CheckinOutcome outcome = new();
        using CancellationTokenSource budget = new(Budget(timeoutMs, PathBudgetFactor + 1d));
        using CancellationTokenSource? linkedDeadline = outerDeadline is null
            ? null
            : CancellationTokenSource.CreateLinkedTokenSource(budget.Token, outerDeadline.Value);
        CancellationToken deadline = linkedDeadline?.Token ?? budget.Token;

        AuthResult resolved = await Auth.ResolveAsync(site, credential, timeoutMs, deadline).ConfigureAwait(false);
        if (!resolved.Ok) {
            outcome.State = resolved.State;
            outcome.Message = resolved.Message;
            return outcome;
        }
        Dictionary<string, string> headers = new(resolved.Headers, StringComparer.OrdinalIgnoreCase);
        StatusResult status = await Protocol.FetchStatusAsync(site, timeoutMs, cachedMeta, deadline).ConfigureAwait(false);
        outcome.Meta = status.Meta;
        outcome.HttpStatus = status.HttpStatus;
        if (status.State is "offline" or "unsupported" or "disabled") {
            outcome.State = status.State;
            outcome.Message = status.Message;
            return outcome;
        }

        IdentityResult identity = await Protocol.ReadSelfAsync(site, headers, timeoutMs, deadline).ConfigureAwait(false);
        if (identity.State is "offline" or "challenge" or "unauthorized") {
            outcome.State = identity.State;
            outcome.Message = identity.Message;
            return outcome;
        }
        if (identity.State == "success") {
            outcome.UserId = identity.UserId;
            outcome.Account = identity.Account;
        }
        ResolveUser(headers, outcome, credential, cachedMeta);

        CheckinVerdict verdict = await AttemptWithRetryAsync(
            site, credential, headers, outcome, cachedMeta, timeoutMs, deadline).ConfigureAwait(false);
        outcome.State = verdict.State;
        outcome.Message = verdict.Message;
        outcome.Quota = Coerce.ToLong(verdict.Quota, 0, 0, long.MaxValue);
        outcome.CheckinDate = verdict.Date;
        outcome.Path = verdict.Path;
        outcome.HttpStatus = Coerce.ToInt(verdict.HttpStatus, outcome.HttpStatus, 0, 1000);
        if (outcome.Account.Length == 0) {
            outcome.Account = Coerce.ToText(cachedMeta?.Account);
        }
        return outcome;
    }
}
