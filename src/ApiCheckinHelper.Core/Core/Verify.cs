namespace ApiCheckinHelper.Core;

public sealed class VerifyOutcome
{
    public bool Ok { get; init; }

    public string State { get; init; } = "unauthorized";

    public string Message { get; init; } = "";

    public int UserId { get; init; }

    public string Account { get; init; } = "";

}

public static class Verify
{
    public const int MaxMessage = 60;

    public static async Task<VerifyOutcome> RunAsync(
        Site site,
        Credential credential,
        int timeoutMs,
        MetaEntry? cachedMeta = null,
        CancellationToken deadline = default)
    {
        AuthResult resolved = await Auth.ResolveAsync(site, credential, timeoutMs, deadline).ConfigureAwait(false);
        if (!resolved.Ok) {
            return new VerifyOutcome {
                Ok = false,
                State = resolved.State,
                Message = resolved.Message,
            };
        }
        Dictionary<string, string> headers = new(resolved.Headers, StringComparer.OrdinalIgnoreCase);
        int manual = Coerce.ToInt(credential.UserId, 0, 0, 1_000_000_000);
        int known = Coerce.ToInt(cachedMeta?.UserId, 0, 0, 1_000_000_000);
        int candidate = manual > 0 ? manual : known;
        if (candidate > 0) {
            headers[Checkin.UserHeader] = candidate.ToString(System.Globalization.CultureInfo.InvariantCulture);
        }

        IdentityResult identity = await Protocol.ReadSelfAsync(site, headers, timeoutMs, deadline).ConfigureAwait(false);
        if (identity.State == "success") {
            return new VerifyOutcome {
                Ok = true,
                State = "success",
                Message = "凭据可用",
                UserId = identity.UserId,
                Account = identity.Account,
            };
        }
        string state = identity.State;
        string message = identity.Message;
        if (state == "unsupported") {
            StatusResult status = await Protocol
                .FetchStatusAsync(site, timeoutMs, cachedMeta, deadline)
                .ConfigureAwait(false);
            state = status.State is "unsupported" or "offline" ? status.State : "unsupported";
            if (state == "unsupported") {
                message = "凭据没被拒绝，但站点读不出用户信息，请手动确认用户 ID";
            }
        }
        string trimmed = message.Length > MaxMessage ? message[..MaxMessage] : message;
        return new VerifyOutcome {
            Ok = false,
            State = state,
            Message = trimmed,
        };
    }
}
