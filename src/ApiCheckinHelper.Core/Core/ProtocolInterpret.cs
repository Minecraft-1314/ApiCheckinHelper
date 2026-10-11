using System.Text.Json.Nodes;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public static partial class Protocol
{
    private static object? PickQuotaNode(JsonObject? data, JsonNode? payload)
    {
        if (data is not null && data.TryGetPropertyValue("quota_awarded", out JsonNode? awarded)) {
            return J.Raw(awarded);
        }
        if (data is not null && data.TryGetPropertyValue("quota", out JsonNode? plain)) {
            return J.Raw(plain);
        }
        return J.Raw(J.Get(payload, "quota"));
    }

    public static CheckinVerdict Interpret(int status, NetResponse response)
    {
        JsonNode? payload = response.Data;
        if (status == 401) {
            return new CheckinVerdict { State = "unauthorized", Message = "访问令牌无效或已过期" };
        }
        if (status == 403) {
            return new CheckinVerdict { State = "challenge", Message = "被站点风控拦截（403）" };
        }
        if (status == 429) {
            return new CheckinVerdict { State = "failed", Message = "请求过于频繁（429），稍后再试" };
        }
        if (status >= 500) {
            return new CheckinVerdict { State = "failed", Message = $"服务端错误（{status}）" };
        }
        if (payload is null) {
            bool html = status == 200 && response.LooksHtml();
            return new CheckinVerdict {
                State = html ? "unsupported" : "failed",
                Message = html ? "该站点不是 New API 面板" : $"无法解析响应（HTTP {status}）",
            };
        }
        string text = ReadMessage(payload);
        if (ReadSuccess(payload) == true) {
            JsonObject? data = ReadData(payload);
            return new CheckinVerdict {
                State = "success",
                Message = text.Length > 0 ? text : "签到成功",
                Quota = Coerce.ToLong(PickQuotaNode(data, payload), 0, 0, long.MaxValue),
                Date = data is null ? "" : J.Str(data, "checkin_date"),
            };
        }
        if (NeedsTokenRegex().IsMatch(text)) {
            return new CheckinVerdict { State = "challenge", Message = FirstNonEmpty(text, "该站点要求人机验证，无法自动完成") };
        }
        if (AlreadyRegex().IsMatch(text)) {
            return new CheckinVerdict { State = "already", Message = FirstNonEmpty(text, "今日已签到") };
        }
        if (DisabledRegex().IsMatch(text)) {
            return new CheckinVerdict { State = "disabled", Message = FirstNonEmpty(text, "站点未开放签到") };
        }
        if (AuthRegex().IsMatch(text)) {
            return new CheckinVerdict { State = "unauthorized", Message = FirstNonEmpty(text, "访问令牌无效") };
        }
        if (MissingEndpointRegex().IsMatch(text)) {
            return new CheckinVerdict { State = "skip", Message = text };
        }
        return new CheckinVerdict { State = "failed", Message = FirstNonEmpty(text, $"签到失败（HTTP {status}）") };
    }
}
