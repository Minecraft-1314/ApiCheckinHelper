namespace ApiCheckinHelper.Core;

public sealed class AuthResult
{
    public bool Ok { get; init; }

    public string State { get; init; } = "unauthorized";

    public string Message { get; init; } = "";

    public IReadOnlyDictionary<string, string> Headers { get; init; } = new Dictionary<string, string>();

    public static AuthResult Fail(string state, string message) => new()
    {
        Ok = false,
        State = state,
        Message = message,
    };

    public static AuthResult Bearer(string token) => new()
    {
        Ok = true,
        State = "success",
        Message = "凭据可用",
        Headers = new Dictionary<string, string>
        {
            ["Authorization"] = "Bearer " + token,
        },
    };
}

public static class Auth
{
    public static string StripBearer(string? value)
    {
        string text = Coerce.ToText(value).Trim();
        return text.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase)
            ? text[7..].Trim()
            : text;
    }

    public static Dictionary<string, string> SiteHeaders(Site site, string path = "/")
    {
        return new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        {
            ["Accept"] = "application/json",
            ["Content-Type"] = "application/json",
            ["Origin"] = site.Origin,
            ["Referer"] = site.Origin + path,
        };
    }

    public static Task<AuthResult> ResolveAsync(
        Site site,
        Credential credential,
        int timeoutMs,
        CancellationToken deadline)
    {
        _ = site;
        _ = timeoutMs;
        _ = deadline;
        string token = StripBearer(credential.Token);
        return Task.FromResult(token.Length > 0
            ? AuthResult.Bearer(token)
            : AuthResult.Fail("unauthorized", "未配置访问令牌"));
    }
}
