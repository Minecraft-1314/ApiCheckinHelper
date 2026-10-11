using System.Collections.Specialized;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Web;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public static partial class OAuthFlows
{
    public static async Task<JsonObject?> StatusDataAsync(Site site, int timeoutMs, CancellationToken deadline = default)
    {
        foreach (string origin in Protocol.CandidateOrigins(site))
        {
            NetResponse response;
            try
            {
                response = await Net.GetAsync($"{origin}/api/status", timeoutMs, null, deadline).ConfigureAwait(false);
            }
            catch (NetError)
            {
                continue;
            }
            if (response.Status < 400)
            {
                JsonObject? data = Protocol.ReadData(response.Data);
                if (data is not null)
                {
                    return data;
                }
            }
        }
        return null;
    }

    public static async Task<OAuthFlow> StartAsync(Site site, string slug, int timeoutMs, CancellationToken deadline = default)
    {
        JsonObject? data = await StatusDataAsync(site, timeoutMs, deadline).ConfigureAwait(false);
        if (data is null)
        {
            return new OAuthFlow { Ok = false, State = "offline", Message = "站点不可用或不是 New API 面板" };
        }
        OAuthProvider? provider = OAuth.ReadProviders(data).FirstOrDefault(item => item.Slug == slug);
        if (provider is null)
        {
            return new OAuthFlow { Ok = false, State = "disabled", Message = "该站点未开启这个登录方式" };
        }
        Dictionary<string, string> headers = Auth.SiteHeaders(site, "/login");
        Dictionary<string, string> body = new(StringComparer.Ordinal)
        {
            ["provider"] = slug,
            ["intent"] = "login",
        };
        NetResponse response;
        try
        {
            response = await Net.PostAsync($"{site.Origin}/api/oauth/state", timeoutMs, headers, body, deadline).ConfigureAwait(false);
        }
        catch (NetError error)
        {
            return new OAuthFlow { Ok = false, State = "offline", Message = error.Message };
        }
        if (response.Status is 404 or 405)
        {
            return new OAuthFlow { Ok = false, State = "unsupported", Message = "该站点版本较旧，没有可用的 OAuth 授权接口" };
        }
        JsonObject? resData = Protocol.ReadData(response.Data);
        string flowToken = J.Str(resData, "flow_token");
        if (flowToken.Length == 0)
        {
            return new OAuthFlow
            {
                Ok = false,
                State = "failed",
                Message = Protocol.FirstNonEmpty(Protocol.ReadMessage(response.Data), "未能获取授权状态"),
            };
        }
        string authorize = J.Str(resData, "authorization_url");
        if (authorize.Length == 0)
        {
            authorize = OAuth.BuildAuthorizeUrl(site, provider, flowToken, data);
        }
        if (authorize.Length == 0)
        {
            return new OAuthFlow
            {
                Ok = false,
                State = "failed",
                Message = "这个登录方式缺少可用的授权地址，请改用访问令牌或 Cookie",
            };
        }
        return new OAuthFlow
        {
            Ok = true,
            State = "success",
            Name = provider.Name,
            Slug = slug,
            FlowToken = flowToken,
            ExpiresAt = J.Long(resData, "expires_at", 0),
            AuthorizeUrl = authorize,
            RedirectUri = $"{site.Origin}/oauth/{slug}",
        };
    }

    public static (string Code, string State)? ParseCallback(string? text)
    {
        string raw = Coerce.ToText(text).Trim();
        if (raw.Length == 0)
        {
            return null;
        }
        if (raw.StartsWith('{'))
        {
            try
            {
                if (JsonNode.Parse(raw) is JsonObject map)
                {
                    string code = J.Str(map, "code");
                    if (code.Length > 0)
                    {
                        return (code, J.Str(map, "state"));
                    }
                }
            }
            catch (JsonException)
            {
                return null;
            }
            return null;
        }
        string query;
        int question = raw.IndexOf('?');
        if (question >= 0)
        {
            query = raw[(question + 1)..];
        }
        else if (raw.StartsWith("code=", StringComparison.Ordinal) || raw.StartsWith("state=", StringComparison.Ordinal))
        {
            query = raw;
        }
        else
        {
            return null;
        }
        NameValueCollection parsed = HttpUtility.ParseQueryString(query);
        string codeValue = parsed["code"] ?? "";
        if (codeValue.Length == 0)
        {
            return null;
        }
        return (codeValue, parsed["state"] ?? "");
    }

    private static OAuthToken Fail(string message) => new() { Ok = false, State = "failed", Message = message };

    public static async Task<OAuthToken> FinishAsync(
        Site site,
        string slug,
        string? text,
        string expectedState,
        int timeoutMs,
        CancellationToken deadline = default)
    {
        (string Code, string State)? parsed = ParseCallback(text);
        if (parsed is null)
        {
            return Fail("没识别出授权码，请把地址栏里带 code 参数的整段地址粘进来");
        }
        if (parsed.Value.State.Length == 0)
        {
            return Fail("地址里缺少 state 参数，请确认复制的是完整的回调地址");
        }
        if (expectedState.Length > 0 && parsed.Value.State != expectedState)
        {
            return Fail("state 与本次发起的授权不一致，请重新从本窗口发起一次");
        }
        string query = $"code={HttpUtility.UrlEncode(parsed.Value.Code)}&state={HttpUtility.UrlEncode(parsed.Value.State)}";
        string url = $"{site.Origin}/api/oauth/{HttpUtility.UrlEncode(slug)}?{query}";
        NetResponse response;
        try
        {
            response = await Net.GetAsync(url, timeoutMs, Auth.SiteHeaders(site, $"/oauth/{slug}"), deadline).ConfigureAwait(false);
        }
        catch (NetError error)
        {
            return new OAuthToken { Ok = false, State = "offline", Message = error.Message };
        }
        JsonObject? data = Protocol.ReadData(response.Data);
        string token = J.Str(data, "access_token");
        if (token.Length == 0)
        {
            return new OAuthToken
            {
                Ok = false,
                State = "unauthorized",
                Message = Protocol.FirstNonEmpty(Protocol.ReadMessage(response.Data), "站点未返回访问令牌"),
            };
        }
        string refresh = response.Cookie(Protocol.RefreshCookie) ?? response.Cookie("session") ?? "";
        JsonObject? user = J.Obj(data, "user");
        return new OAuthToken
        {
            Ok = true,
            State = "success",
            AccessToken = token,
            AccessExpiresAt = J.Long(data, "access_expires_at", 0),
            RefreshCookie = refresh,
            Account = user is null ? "" : Protocol.FirstNonEmpty(J.Str(user, "display_name"), J.Str(user, "username")),
        };
    }

    public static async Task<DiscoveryResult> DiscoverAsync(string url, int timeoutMs, CancellationToken deadline = default)
    {
        Site? site = Model.NormalizeSite(new Site { Host = url });
        if (site is null)
        {
            return new DiscoveryResult { Ok = false, Message = "地址格式不合法，请填写形如 api.example.com 的域名" };
        }
        Site? existing = ConfigStore.FindByHost(site.Host);
        if (existing is not null)
        {
            return new DiscoveryResult
            {
                Ok = true,
                Exists = true,
                Host = existing.Host,
                SiteId = existing.Id,
                Name = existing.Name,
            };
        }
        JsonObject? data = await StatusDataAsync(site, timeoutMs, deadline).ConfigureAwait(false);
        if (data is null)
        {
            return new DiscoveryResult { Ok = false, Host = site.Host, Message = "访问不到这个地址，或者它不是 New API 面板" };
        }
        List<string> names = OAuth.ReadProviders(data).Select(item => item.Name).ToList();
        bool passwordLogin = !data.ContainsKey("password_login_enabled") || J.Bool(data, "password_login_enabled");
        return new DiscoveryResult
        {
            Ok = true,
            Exists = false,
            Host = site.Host,
            Origin = site.Origin,
            Name = Protocol.FirstNonEmpty(J.Str(data, "system_name"), site.Host),
            Version = J.Str(data, "version"),
            CheckinEnabled = J.Bool(data, "checkin_enabled"),
            Turnstile = J.Bool(data, "turnstile_check"),
            PasswordLogin = passwordLogin,
            OAuth = names,
        };
    }
}
