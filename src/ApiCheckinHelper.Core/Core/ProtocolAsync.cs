using System.Text.Json.Nodes;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public static partial class Protocol
{
    private static StatusResult FromMeta(SiteSnapshot? meta, string origin, int httpStatus)
    {
        if (meta is null) {
            return new StatusResult {
                State = "unsupported",
                Message = "该站点不是 New API 面板",
                Origin = origin,
                HttpStatus = httpStatus,
            };
        }
        if (meta.CheckinEnabled == false) {
            return new StatusResult {
                State = "disabled",
                Message = "站点未开启签到功能",
                Meta = meta,
                Origin = origin,
                HttpStatus = httpStatus,
            };
        }
        return new StatusResult {
            State = "idle",
            Message = meta.Turnstile ? "需人机验证，仅能手动完成" : "可签到",
            Meta = meta,
            Origin = origin,
            HttpStatus = httpStatus,
        };
    }

    private static async Task<StatusResult> FetchStatusOnceAsync(string origin, int timeoutMs, CancellationToken deadline)
    {
        NetResponse response;
        try {
            response = await Net.GetAsync($"{origin}/api/status", timeoutMs, null, deadline).ConfigureAwait(false);
        } catch (NetError error) {
            return new StatusResult { State = "offline", Message = error.Message };
        }
        if (response.Status >= 500) {
            return new StatusResult {
                State = "offline",
                Message = $"站点返回 {response.Status}",
                HttpStatus = response.Status,
                Origin = origin,
            };
        }
        if (response.Status >= 400) {
            return new StatusResult {
                State = "offline",
                Message = $"站点不可用（HTTP {response.Status}）",
                HttpStatus = response.Status,
                Origin = origin,
            };
        }
        SiteSnapshot? meta = ReadMeta(response.Data);
        if (meta is null) {
            return new StatusResult {
                State = "unsupported",
                Message = "该站点不是 New API 面板",
                HttpStatus = response.Status,
                Origin = origin,
            };
        }
        return FromMeta(meta, origin, response.Status);
    }

    public static async Task<StatusResult> FetchStatusAsync(
        Site site,
        int timeoutMs,
        MetaEntry? cachedMeta = null,
        CancellationToken deadline = default)
    {
        bool fresh = cachedMeta is not null
            && cachedMeta.ProbedAt > 0
            && DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - cachedMeta.ProbedAt < StatusTtlMs;
        if (fresh) {
            SiteSnapshot? view = BuildMetaView(cachedMeta);
            if (view is not null) {
                return FromMeta(view, site.Origin, 200);
            }
        }
        StatusResult result = new() { State = "offline", Message = "无法连接站点" };
        foreach (string origin in CandidateOrigins(site)) {
            result = await FetchStatusOnceAsync(origin, timeoutMs, deadline).ConfigureAwait(false);
            bool done = result.State != "offline";
            bool expired = deadline.CanBeCanceled && deadline.IsCancellationRequested;
            if (done || expired) {
                break;
            }
        }
        return result;
    }

    private static IdentityResult? InterpretSelfStatus(int status, JsonNode? payload)
    {
        if (status == 404) {
            return null;
        }
        if (status == 401) {
            return new IdentityResult { State = "unauthorized", Message = "访问令牌无效或已过期" };
        }
        if (status == 403) {
            return new IdentityResult { State = "challenge", Message = "站点风控拦截，令牌请求未被接受" };
        }
        if (status >= 500) {
            return new IdentityResult { State = "offline", Message = $"站点返回 {status}" };
        }
        string message = ReadMessage(payload);
        string code = ReadCode(payload);
        bool unauthorized = ReadSuccess(payload) == false
            && (AuthRegex().IsMatch(message) || string.Equals(code, "AUTH_UNAUTHORIZED", StringComparison.Ordinal));
        if (unauthorized) {
            return new IdentityResult {
                State = "unauthorized",
                Message = message.Length > 0 ? message : "访问令牌无效或已过期",
            };
        }
        return null;
    }

    public static async Task<IdentityResult> ReadSelfAsync(
        Site site,
        IReadOnlyDictionary<string, string> headers,
        int timeoutMs,
        CancellationToken deadline = default)
    {
        foreach (string path in SelfPaths) {
            NetResponse response;
            try {
                response = await Net.GetAsync($"{site.Origin}{path}", timeoutMs, headers, deadline).ConfigureAwait(false);
            } catch (NetError error) {
                return new IdentityResult { State = "offline", Message = error.Message };
            }
            IdentityResult? shortCircuit = InterpretSelfStatus(response.Status, response.Data);
            if (shortCircuit is not null) {
                return shortCircuit;
            }
            if (response.Status == 404) {
                continue;
            }
            JsonObject? data = ReadData(response.Data);
            if (data is null) {
                continue;
            }
            return new IdentityResult {
                State = "success",
                Message = "凭据可用",
                UserId = J.Int(data, "id", 0, 0, 1_000_000_000),
                Account = FirstNonEmpty(J.Str(data, "display_name"), J.Str(data, "username")),
            };
        }
        return new IdentityResult { State = "unsupported", Message = "旧版本站点没有可用的用户信息接口" };
    }
}
