using System.Text.Json;
using ApiCheckinHelper.Core;

namespace ApiCheckinHelper.Ui.Bridge;

public sealed class CommandRouter(Action<object> postEvent) : IBridgeHost, IDisposable
{
    private const int MaxAddressLength = 2048;
    private const int MaxSiteIdLength = 128;
    private const int MaxNameLength = 80;
    private const int MaxShortValueLength = 32;

    private readonly OperationGate _operations = new();
    private int _progressCount;

    public void Dispose() => _operations.Dispose();

    public OperationLease CreateOperation() => _operations.Create();

    public void PostEvent(object payload) => postEvent(payload);

    public async Task<object> ExecuteAsync(string action, JsonElement payload)
    {
        object? result = action switch {
            "getState" => StateView.Build(BridgeJson.Bool(payload, "refresh")),
            "openExternal" => ExternalLauncher.Open(ReadText(payload, "url", MaxAddressLength)),
            "openDataDirectory" => ExternalLauncher.OpenDataDirectory(null),
            "addSite" => AddSite(payload),
            "discoverSite" => await DiscoverAsync(payload).ConfigureAwait(true),
            "probeSite" => await ProbeSiteAsync(payload).ConfigureAwait(true),
            "renameSite" => RenameSite(payload),
            "removeSite" => RemoveSite(payload),
            "saveCredential" => SaveCredential(payload),
            "clearCredential" => ClearCredential(payload),
            "clearAllCredentials" => ClearAllCredentials(),
            "verifySite" => await VerifySiteAsync(payload).ConfigureAwait(true),
            "verifyAll" => await VerifyAllAsync().ConfigureAwait(true),
            "importCredentials" => ImportCredentials(payload),
            "runCheckin" => await RunCheckinAsync(payload).ConfigureAwait(true),
            "patchSettings" => PatchSettings(payload),
            "setTheme" => SetTheme(payload),
            "cancel" => Cancel(),
            _ => throw new ConfigError("未知的界面操作：" + action),
        };
        return result!;
    }

    private static object AddSite(JsonElement payload)
    {
        Site site = ConfigStore.AddSite(
            ReadText(payload, "host", MaxAddressLength),
            ReadText(payload, "name", MaxNameLength));
        return StateView.Of(site, ConfigStore.Load());
    }

    private async Task<object> DiscoverAsync(JsonElement payload)
    {
        Settings settings = ConfigStore.Settings();
        using OperationLease lease = CreateOperation();
        return await OAuthFlows
            .DiscoverAsync(ReadText(payload, "host", MaxAddressLength), settings.TimeoutMs, lease.Token)
            .ConfigureAwait(true);
    }

    private async Task<object> ProbeSiteAsync(JsonElement payload)
    {
        using OperationLease lease = CreateOperation();
        CheckinOutcome outcome = await Runner
            .ProbeAsync(ReadText(payload, "siteId", MaxSiteIdLength), lease.Token)
            .ConfigureAwait(true);
        return new
        {
            outcome.State,
            outcome.Message,
            outcome.HttpStatus,
        };
    }

    private static object RenameSite(JsonElement payload)
    {
        Site site = ConfigStore.UpdateSite(
            ReadText(payload, "siteId", MaxSiteIdLength),
            null,
            ReadText(payload, "name", MaxNameLength));
        return StateView.Of(site, ConfigStore.Load());
    }

    private static bool RemoveSite(JsonElement payload)
    {
        string siteId = ReadText(payload, "siteId", MaxSiteIdLength);
        ConfigStore.RemoveSite(siteId);
        Protocol.Invalidate(siteId);
        return true;
    }

    private static bool SaveCredential(JsonElement payload)
    {
        string siteId = ReadText(payload, "siteId", MaxSiteIdLength);
        Credential credential = new()
        {
            Type = "token",
            Token = ReadText(payload, "token", Model.MaxTokenLength),
            UserId = BridgeJson.Int(payload, "userId"),
        };
        if (!Model.IsConfigured(credential))
        {
            throw new ConfigError("请填写访问令牌");
        }
        ConfigStore.SaveCredential(siteId, credential);
        return true;
    }

    private static bool ClearCredential(JsonElement payload)
    {
        ConfigStore.ClearCredential(ReadText(payload, "siteId", MaxSiteIdLength));
        return true;
    }

    private static bool ClearAllCredentials()
    {
        ConfigStore.ClearCredentials();
        return true;
    }

    private async Task<object> VerifySiteAsync(JsonElement payload)
    {
        string siteId = ReadText(payload, "siteId", MaxSiteIdLength);
        Site? site = ConfigStore.FindSite(siteId);
        if (site is null)
        {
            throw new ConfigError("站点不存在");
        }
        Credential credential = ConfigStore.CredentialsFor(siteId);
        MetaEntry meta = ConfigStore.MetaFor(siteId);
        Settings settings = ConfigStore.Settings();
        using OperationLease lease = CreateOperation();
        VerifyOutcome outcome = await Verify
            .RunAsync(site, credential, settings.TimeoutMs, meta, lease.Token)
            .ConfigureAwait(true);
        if (outcome.Ok)
        {
            ConfigStore.SetMeta(siteId, new MetaEntry
            {
                Account = outcome.Account.Length > 0 ? outcome.Account : meta.Account,
                UserId = outcome.UserId > 0 ? outcome.UserId : meta.UserId,
            });
        }
        return new
        {
            ok = outcome.Ok,
            state = outcome.State,
            message = outcome.Message,
            userId = outcome.UserId,
            account = outcome.Account,
        };
    }

    private async Task<object> VerifyAllAsync()
    {
        using OperationLease lease = CreateOperation();
        VerifySummary summary = await Runner.RunVerifyAsync(null, lease.Token).ConfigureAwait(true);
        return new
        {
            summary.Total,
            summary.Ok,
            summary.Failed,
            summary.Items,
        };
    }

    private static CredentialImportResult ImportCredentials(JsonElement payload)
    {
        if (!payload.TryGetProperty("entries", out JsonElement entries) || entries.ValueKind != JsonValueKind.Array)
        {
            throw new ConfigError("缺少导入内容");
        }
        if (entries.GetArrayLength() > Model.MaxImportEntries)
        {
            throw new ConfigError($"一次最多导入 {Model.MaxImportEntries} 条凭据");
        }
        List<CredentialImportEntry> imports = new();
        foreach (JsonElement node in entries.EnumerateArray())
        {
            imports.Add(new CredentialImportEntry
            {
                Name = ReadText(node, "name", MaxNameLength).Trim(),
                Host = ReadText(node, "host", MaxAddressLength).Trim(),
                Type = ReadText(node, "type", MaxShortValueLength, "token").Trim(),
                Token = ReadText(node, "token", Model.MaxTokenLength),
                UserId = BridgeJson.Int(node, "userId"),
            });
        }
        return CredentialImport.Apply(imports);
    }

    private async Task<object> RunCheckinAsync(JsonElement payload)
    {
        string siteId = ReadText(payload, "siteId", MaxSiteIdLength);
        using OperationLease lease = CreateOperation();
        Interlocked.Exchange(ref _progressCount, 0);
        int total = siteId.Length == 0
            ? ConfigStore.Sites().Count
            : ConfigStore.FindSite(siteId) is null ? 0 : 1;
        BridgeProgress progress = new(result => ReportProgress(result, total));
        CheckinSummary summary = await Runner
            .RunCheckinAsync(siteId.Length == 0 ? null : siteId, progress, lease.Token)
            .ConfigureAwait(true);
        return new
        {
            summary.Total,
            summary.ElapsedMs,
            summary.Results,
        };
    }

    private void ReportProgress(RunResult result, int total)
    {
        int done = Interlocked.Increment(ref _progressCount);
        PostEvent(new
        {
            type = "progress",
            kind = "checkin",
            done,
            total,
            label = $"{done}/{total}",
            result,
        });
    }

    private static object PatchSettings(JsonElement payload)
    {
        ConfigStore.PatchSettings(ReadSettings(payload));
        return StateView.Build(false);
    }

    private static object SetTheme(JsonElement payload)
    {
        Settings current = ConfigStore.Settings();
        current.Theme = ReadText(payload, "theme", MaxShortValueLength);
        ConfigStore.PatchSettings(current);
        return StateView.Build(false);
    }

    private bool Cancel()
    {
        _operations.CancelCurrent();
        return true;
    }

    private static Settings ReadSettings(JsonElement payload)
    {
        Settings current = ConfigStore.Settings();
        return Model.NormalizeSettings(new Settings
        {
            Concurrency = BridgeJson.Int(payload, "concurrency", current.Concurrency),
            TimeoutMs = BridgeJson.Int(payload, "timeoutMs", current.TimeoutMs),
            Notify = BridgeJson.Bool(payload, "notify", current.Notify),
            SkipManualOnly = BridgeJson.Bool(payload, "skipManualOnly", current.SkipManualOnly),
            Theme = ReadText(payload, "theme", MaxShortValueLength, current.Theme),
            Language = ReadText(payload, "language", MaxShortValueLength, current.Language),
        });
    }

    private sealed class BridgeProgress(Action<RunResult> report) : IProgress<RunResult>
    {
        public void Report(RunResult value) => report(value);
    }

    private static string ReadText(JsonElement source, string name, int maximumLength, string fallback = "")
    {
        string value = BridgeJson.Text(source, name, fallback);
        if (value.Length > maximumLength)
        {
            throw new ConfigError($"字段 {name} 超过长度上限");
        }
        return value;
    }
}
