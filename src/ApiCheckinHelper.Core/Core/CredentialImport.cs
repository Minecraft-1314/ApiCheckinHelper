namespace ApiCheckinHelper.Core;

public sealed class CredentialImportEntry
{
    public string Name { get; init; } = "";

    public string Host { get; init; } = "";

    public string Type { get; init; } = "token";

    public string Token { get; init; } = "";

    public int UserId { get; init; }
}

public sealed class CredentialImportResult
{
    public int Imported { get; init; }

    public int AddedSites { get; init; }

    public List<string> Errors { get; init; } = new();
}

public static class CredentialImport
{
    public static CredentialImportResult Apply(IEnumerable<CredentialImportEntry>? entries)
    {
        List<string> errors = new();
        List<(Site Site, Credential Credential, int Row)> drafts = new();
        HashSet<string> hosts = new(StringComparer.OrdinalIgnoreCase);
        int rowNumber = 0;

        foreach (CredentialImportEntry entry in entries ?? Enumerable.Empty<CredentialImportEntry>())
        {
            rowNumber++;
            Site? site = Model.NormalizeSite(new Site
            {
                Host = entry.Host,
                Name = entry.Name,
            });
            if (site is null)
            {
                errors.Add($"第 {rowNumber} 条：站点地址不合法");
                continue;
            }
            if (!hosts.Add(site.Host))
            {
                errors.Add($"第 {rowNumber} 条：站点地址重复");
                continue;
            }
            if (!string.Equals(entry.Type.Trim(), "token", StringComparison.OrdinalIgnoreCase))
            {
                errors.Add($"第 {rowNumber} 条：凭据类型不受支持");
                continue;
            }
            Credential credential = Model.NormalizeCredential(new Credential
            {
                Type = "token",
                Token = entry.Token,
                UserId = entry.UserId,
            });
            if (!Model.IsConfigured(credential))
            {
                errors.Add($"第 {rowNumber} 条：访问令牌为空");
                continue;
            }
            drafts.Add((site, credential, rowNumber));
        }

        Dictionary<string, Credential> patches = new(StringComparer.Ordinal);
        int addedSites = 0;
        foreach ((Site draftSite, Credential credential, int row) in drafts)
        {
            try
            {
                if (ConfigStore.FindByHost(draftSite.Host) is not null)
                {
                    errors.Add($"第 {row} 条：站点已存在，请直接编辑该站点");
                    continue;
                }
                Site site = ConfigStore.AddSite(draftSite.Host, draftSite.Name);
                patches[site.Id] = credential;
                addedSites++;
            }
            catch (ConfigError error)
            {
                errors.Add($"第 {row} 条：{error.Message}");
            }
        }
        if (patches.Count > 0)
        {
            ConfigStore.SetCredentials(patches);
        }

        return new CredentialImportResult
        {
            Imported = patches.Count,
            AddedSites = addedSites,
            Errors = errors,
        };
    }
}
