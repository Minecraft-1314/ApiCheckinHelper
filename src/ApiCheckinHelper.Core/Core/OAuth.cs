using System.Text;
using System.Text.Json.Nodes;
using System.Web;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public sealed class OAuthProvider
{
    public string Slug { get; init; } = "";

    public string Name { get; init; } = "";

    public string Kind { get; init; } = "";

    public string ClientId { get; init; } = "";

    public string Endpoint { get; init; } = "";

    public string Scopes { get; init; } = "";
}

public sealed class OAuthFlow
{
    public bool Ok { get; init; }

    public string State { get; init; } = "";

    public string Message { get; init; } = "";

    public string Name { get; init; } = "";

    public string Slug { get; init; } = "";

    public string FlowToken { get; init; } = "";

    public long ExpiresAt { get; init; }

    public string AuthorizeUrl { get; init; } = "";

    public string RedirectUri { get; init; } = "";
}

public sealed class OAuthToken
{
    public bool Ok { get; init; }

    public string State { get; init; } = "";

    public string Message { get; init; } = "";

    public string AccessToken { get; init; } = "";

    public long AccessExpiresAt { get; init; }

    public string RefreshCookie { get; init; } = "";

    public string Account { get; init; } = "";
}

public sealed class DiscoveryResult
{
    public bool Ok { get; init; }

    public bool Exists { get; init; }

    public string Host { get; init; } = "";

    public string Origin { get; init; } = "";

    public string SiteId { get; init; } = "";

    public string Name { get; init; } = "";

    public string Version { get; init; } = "";

    public string Message { get; init; } = "";

    public bool CheckinEnabled { get; init; }

    public bool Turnstile { get; init; }

    public bool PasswordLogin { get; init; }

    public List<string> OAuth { get; init; } = new();
}

public static class OAuth
{
    private sealed record Builtin(string Slug, string Name, string Flag, string? IdField);

    private static readonly Builtin[] Known =
    {
        new("github", "GitHub", "github_oauth", "github_client_id"),
        new("discord", "Discord", "discord_oauth", "discord_client_id"),
        new("linuxdo", "LinuxDO", "linuxdo_oauth", "linuxdo_client_id"),
        new("telegram", "Telegram", "telegram_oauth", null),
        new("oidc", "OIDC", "oidc_enabled", "oidc_client_id"),
    };

    private static string BuildQuery(IEnumerable<KeyValuePair<string, string>> values)
    {
        StringBuilder builder = new();
        foreach (KeyValuePair<string, string> pair in values)
        {
            if (builder.Length > 0)
            {
                builder.Append('&');
            }
            builder.Append(HttpUtility.UrlEncode(pair.Key)).Append('=').Append(HttpUtility.UrlEncode(pair.Value));
        }
        return builder.ToString();
    }

    public static List<OAuthProvider> ReadProviders(JsonObject? status)
    {
        List<OAuthProvider> providers = new();
        if (status is null)
        {
            return providers;
        }
        foreach (Builtin item in Known)
        {
            if (J.Bool(status, item.Flag) != true)
            {
                continue;
            }
            string clientId = item.IdField is null ? "" : J.Str(status, item.IdField);
            if (item.IdField is not null && clientId.Length == 0)
            {
                continue;
            }
            if (item.Slug == "oidc" && J.Str(status, "oidc_authorization_endpoint").Length == 0)
            {
                continue;
            }
            providers.Add(new OAuthProvider {
                Slug = item.Slug,
                Name = item.Name,
                Kind = item.Slug,
                ClientId = clientId,
            });
        }
        if (J.Get(status, "custom_oauth_providers") is JsonArray custom)
        {
            foreach (JsonNode? entry in custom)
            {
                if (entry is not JsonObject map)
                {
                    continue;
                }
                string slug = J.Str(map, "slug");
                if (slug.Length == 0)
                {
                    continue;
                }
                string clientId = J.Str(map, "client_id");
                string endpoint = J.Str(map, "authorization_endpoint");
                if (clientId.Length == 0 || endpoint.Length == 0)
                {
                    continue;
                }
                providers.Add(new OAuthProvider {
                    Slug = slug,
                    Name = Protocol.FirstNonEmpty(J.Str(map, "name"), slug),
                    Kind = "custom",
                    ClientId = clientId,
                    Endpoint = endpoint,
                    Scopes = J.Str(map, "scopes"),
                });
            }
        }
        return providers;
    }

    public static string BuildAuthorizeUrl(Site site, OAuthProvider provider, string state, JsonObject? status)
    {
        string slug = provider.Slug;
        string redirect = $"{site.Origin}/oauth/{slug}";
        string clientId = provider.ClientId;
        if (clientId.Length == 0)
        {
            return "";
        }
        if (slug == "github")
        {
            Dictionary<string, string> query = new()
            {
                ["client_id"] = clientId,
                ["redirect_uri"] = redirect,
                ["state"] = state,
                ["scope"] = "user:email",
            };
            return "https://github.com/login/oauth/authorize?" + BuildQuery(query);
        }
        if (slug == "linuxdo")
        {
            Dictionary<string, string> query = new()
            {
                ["response_type"] = "code",
                ["client_id"] = clientId,
                ["redirect_uri"] = redirect,
                ["state"] = state,
            };
            return "https://connect.linux.do/oauth2/authorize?" + BuildQuery(query);
        }
        if (slug == "discord")
        {
            Dictionary<string, string> query = new()
            {
                ["client_id"] = clientId,
                ["redirect_uri"] = redirect,
                ["response_type"] = "code",
                ["scope"] = "identify+openid",
                ["state"] = state,
            };
            return "https://discord.com/oauth2/authorize?" + BuildQuery(query);
        }
        if (slug == "oidc")
        {
            string endpoint = J.Str(status, "oidc_authorization_endpoint");
            if (endpoint.Length == 0)
            {
                return "";
            }
            Dictionary<string, string> query = new()
            {
                ["client_id"] = clientId,
                ["redirect_uri"] = redirect,
                ["response_type"] = "code",
                ["scope"] = "openid profile email",
                ["state"] = state,
            };
            return endpoint + "?" + BuildQuery(query);
        }
        if (provider.Endpoint.Length == 0)
        {
            return "";
        }
        Dictionary<string, string> custom = new()
        {
            ["client_id"] = clientId,
            ["redirect_uri"] = redirect,
            ["response_type"] = "code",
            ["state"] = state,
        };
        if (provider.Scopes.Length > 0)
        {
            custom["scope"] = provider.Scopes;
        }
        return provider.Endpoint + "?" + BuildQuery(custom);
    }
}