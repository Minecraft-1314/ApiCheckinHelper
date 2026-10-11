using ApiCheckinHelper.Core;

namespace ApiCheckinHelper.Ui.Bridge;

public static class StateView
{
    public static object Build(bool refresh)
    {
        AppState state = ConfigStore.Load(refresh);
        return new
        {
            sites = state.Sites.Select(site => Of(site, state)).ToList(),
            settings = state.Settings,
            history = state.History,
            verify = state.Verify,
            theme = state.Settings.Theme,
            language = state.Settings.Language,
            dataDirectory = ConfigStore.DataDirectory,
            lastError = ConfigStore.LastError,
        };
    }

    public static object Of(Site site, AppState state)
    {
        state.Credentials.TryGetValue(site.Id, out Credential? credential);
        state.Meta.TryGetValue(site.Id, out MetaEntry? meta);
        state.Results.TryGetValue(site.Id, out RunResult? result);
        return new
        {
            id = site.Id,
            name = site.Name,
            host = site.Host,
            origin = site.Origin,
            entry = site.Entry,
            bookmark = site.Bookmark,
            configured = Model.IsConfigured(credential),
            credential = credential ?? new Credential(),
            meta = meta ?? new MetaEntry(),
            snapshot = site.Snapshot,
            result,
        };
    }
}
