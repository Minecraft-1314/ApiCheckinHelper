using System.Text.Json;

namespace ApiCheckinHelper.Core.Json;

internal static class ConfigSchema
{
    public static void Validate(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json, new JsonDocumentOptions
        {
            AllowTrailingCommas = false,
            CommentHandling = JsonCommentHandling.Disallow,
            MaxDepth = 64,
        });
        ValidateRoot(document.RootElement);
    }

    private static void ValidateRoot(JsonElement root)
    {
        RequireShape(
            root,
            "$",
            "Version",
            "Settings",
            "Sites",
            "Credentials",
            "Meta",
            "Results",
            "LastRun",
            "Verify",
            "History");
        RequireInt32(Required(root, "Version", "$"), "$.Version", Model.StateVersion);
        ValidateSettings(Required(root, "Settings", "$"), "$.Settings");
        ValidateArray(Required(root, "Sites", "$"), "$.Sites", ValidateSite);
        ValidateMap(Required(root, "Credentials", "$"), "$.Credentials", ValidateCredential, false);
        ValidateMap(Required(root, "Meta", "$"), "$.Meta", ValidateMeta, false);
        ValidateMap(Required(root, "Results", "$"), "$.Results", ValidateResult, true);
        ValidateLastRun(Required(root, "LastRun", "$"), "$.LastRun");
        ValidateVerify(Required(root, "Verify", "$"), "$.Verify");
        ValidateMap(Required(root, "History", "$"), "$.History", ValidateHistory, false);
    }

    private static void ValidateSettings(JsonElement value, string path)
    {
        RequireShape(
            value,
            path,
            "Concurrency",
            "TimeoutMs",
            "Notify",
            "SkipManualOnly",
            "Theme",
            "Language");
        RequireInt32(Required(value, "Concurrency", path), path + ".Concurrency");
        RequireInt32(Required(value, "TimeoutMs", path), path + ".TimeoutMs");
        RequireBoolean(Required(value, "Notify", path), path + ".Notify");
        RequireBoolean(Required(value, "SkipManualOnly", path), path + ".SkipManualOnly");
        RequireString(Required(value, "Theme", path), path + ".Theme");
        RequireString(Required(value, "Language", path), path + ".Language");
    }

    private static void ValidateSite(JsonElement value, string path)
    {
        RequireShape(
            value,
            path,
            "Id",
            "Name",
            "Host",
            "Origin",
            "Entry",
            "Tags",
            "Bookmark",
            "Snapshot");
        RequireString(Required(value, "Id", path), path + ".Id");
        RequireString(Required(value, "Name", path), path + ".Name");
        RequireString(Required(value, "Host", path), path + ".Host");
        RequireString(Required(value, "Origin", path), path + ".Origin");
        RequireString(Required(value, "Entry", path), path + ".Entry");
        ValidateStringArray(Required(value, "Tags", path), path + ".Tags");
        RequireString(Required(value, "Bookmark", path), path + ".Bookmark");
        ValidateSnapshot(Required(value, "Snapshot", path), path + ".Snapshot");
    }

    private static void ValidateSnapshot(JsonElement value, string path)
    {
        RequireShape(
            value,
            path,
            "SystemName",
            "Version",
            "CheckinEnabled",
            "Turnstile",
            "QuotaPerUnit",
            "DisplayInCurrency",
            "DisplayType",
            "ExchangeRate");
        RequireString(Required(value, "SystemName", path), path + ".SystemName");
        RequireOptionalString(value, "Version", path);
        RequireOptionalBoolean(value, "CheckinEnabled", path);
        RequireBoolean(Required(value, "Turnstile", path), path + ".Turnstile");
        RequireInt32(Required(value, "QuotaPerUnit", path), path + ".QuotaPerUnit");
        RequireBoolean(Required(value, "DisplayInCurrency", path), path + ".DisplayInCurrency");
        RequireString(Required(value, "DisplayType", path), path + ".DisplayType");
        RequireDouble(Required(value, "ExchangeRate", path), path + ".ExchangeRate");
    }

    private static void ValidateCredential(JsonElement value, string path)
    {
        RequireShape(value, path, "Type", "Token", "UserId");
        RequireString(Required(value, "Type", path), path + ".Type");
        RequireString(Required(value, "Token", path), path + ".Token");
        RequireInt32(Required(value, "UserId", path), path + ".UserId");
    }

    private static void ValidateMeta(JsonElement value, string path)
    {
        RequireShape(
            value,
            path,
            "SystemName",
            "Version",
            "CheckinEnabled",
            "Turnstile",
            "QuotaPerUnit",
            "DisplayInCurrency",
            "DisplayType",
            "ExchangeRate",
            "Account",
            "UserId",
            "ProbedAt");
        RequireString(Required(value, "SystemName", path), path + ".SystemName");
        RequireString(Required(value, "Version", path), path + ".Version");
        RequireOptionalBoolean(value, "CheckinEnabled", path);
        RequireBoolean(Required(value, "Turnstile", path), path + ".Turnstile");
        RequireInt32(Required(value, "QuotaPerUnit", path), path + ".QuotaPerUnit");
        RequireBoolean(Required(value, "DisplayInCurrency", path), path + ".DisplayInCurrency");
        RequireString(Required(value, "DisplayType", path), path + ".DisplayType");
        RequireDouble(Required(value, "ExchangeRate", path), path + ".ExchangeRate");
        RequireString(Required(value, "Account", path), path + ".Account");
        RequireInt32(Required(value, "UserId", path), path + ".UserId");
        RequireInt64(Required(value, "ProbedAt", path), path + ".ProbedAt");
    }

    private static void ValidateResult(JsonElement value, string path)
    {
        if (value.ValueKind == JsonValueKind.Null)
        {
            return;
        }
        RequireShape(
            value,
            path,
            "Id",
            "State",
            "Message",
            "Quota",
            "Path",
            "HttpStatus",
            "At");
        RequireString(Required(value, "Id", path), path + ".Id");
        RequireString(Required(value, "State", path), path + ".State");
        RequireString(Required(value, "Message", path), path + ".Message");
        RequireInt64(Required(value, "Quota", path), path + ".Quota");
        RequireString(Required(value, "Path", path), path + ".Path");
        RequireInt32(Required(value, "HttpStatus", path), path + ".HttpStatus");
        RequireInt64(Required(value, "At", path), path + ".At");
    }

    private static void ValidateLastRun(JsonElement value, string path)
    {
        RequireShape(value, path, "Day", "At");
        RequireString(Required(value, "Day", path), path + ".Day");
        RequireInt64(Required(value, "At", path), path + ".At");
    }

    private static void ValidateVerify(JsonElement value, string path)
    {
        RequireShape(value, path, "At", "Total", "Ok", "Failed", "Items");
        RequireInt64(Required(value, "At", path), path + ".At");
        RequireInt32(Required(value, "Total", path), path + ".Total");
        RequireInt32(Required(value, "Ok", path), path + ".Ok");
        RequireInt32(Required(value, "Failed", path), path + ".Failed");
        ValidateArray(Required(value, "Items", path), path + ".Items", ValidateVerifyEntry);
    }

    private static void ValidateVerifyEntry(JsonElement value, string path)
    {
        RequireShape(value, path, "Id", "Name", "Host", "Ok", "Message");
        RequireString(Required(value, "Id", path), path + ".Id");
        RequireString(Required(value, "Name", path), path + ".Name");
        RequireString(Required(value, "Host", path), path + ".Host");
        RequireBoolean(Required(value, "Ok", path), path + ".Ok");
        RequireString(Required(value, "Message", path), path + ".Message");
    }

    private static void ValidateHistory(JsonElement value, string path)
    {
        RequireShape(
            value,
            path,
            "Total",
            "Success",
            "Already",
            "Failed",
            "Skipped",
            "Quota",
            "QuotaUsd",
            "At");
        RequireInt32(Required(value, "Total", path), path + ".Total");
        RequireInt32(Required(value, "Success", path), path + ".Success");
        RequireInt32(Required(value, "Already", path), path + ".Already");
        RequireInt32(Required(value, "Failed", path), path + ".Failed");
        RequireInt32(Required(value, "Skipped", path), path + ".Skipped");
        RequireInt64(Required(value, "Quota", path), path + ".Quota");
        RequireDouble(Required(value, "QuotaUsd", path), path + ".QuotaUsd");
        RequireInt64(Required(value, "At", path), path + ".At");
    }

    private static void ValidateMap(
        JsonElement value,
        string path,
        Action<JsonElement, string> validateValue,
        bool allowNullValues)
    {
        RequireKind(value, JsonValueKind.Object, path);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            string childPath = path + "." + property.Name;
            if (property.Value.ValueKind == JsonValueKind.Null && !allowNullValues)
            {
                throw Invalid(childPath, "不允许空值");
            }
            validateValue(property.Value, childPath);
        }
    }

    private static void ValidateArray(JsonElement value, string path, Action<JsonElement, string> validateItem)
    {
        RequireKind(value, JsonValueKind.Array, path);
        int index = 0;
        foreach (JsonElement item in value.EnumerateArray())
        {
            validateItem(item, $"{path}[{index}]");
            index++;
        }
    }

    private static void ValidateStringArray(JsonElement value, string path)
    {
        ValidateArray(value, path, (item, itemPath) => RequireString(item, itemPath));
    }

    private static JsonElement Required(JsonElement value, string name, string path)
    {
        if (!value.TryGetProperty(name, out JsonElement result))
        {
            throw Invalid(path, $"缺少字段 {name}");
        }
        return result;
    }

    private static void RequireShape(JsonElement value, string path, params string[] fields)
    {
        RequireKind(value, JsonValueKind.Object, path);
        HashSet<string> allowed = new(fields, StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!allowed.Contains(property.Name))
            {
                throw Invalid(path, $"未知字段 {property.Name}");
            }
        }
    }

    private static void RequireOptionalString(JsonElement value, string name, string path)
    {
        if (!value.TryGetProperty(name, out JsonElement item))
        {
            return;
        }
        RequireString(item, $"{path}.{name}");
    }

    private static void RequireOptionalBoolean(JsonElement value, string name, string path)
    {
        if (!value.TryGetProperty(name, out JsonElement item))
        {
            return;
        }
        RequireBoolean(item, $"{path}.{name}");
    }

    private static void RequireString(JsonElement value, string path)
    {
        RequireKind(value, JsonValueKind.String, path);
    }

    private static void RequireBoolean(JsonElement value, string path)
    {
        RequireKind(value, JsonValueKind.True, path, JsonValueKind.False);
    }

    private static void RequireInt32(JsonElement value, string path, int? expected = null)
    {
        RequireKind(value, JsonValueKind.Number, path);
        if (!value.TryGetInt32(out int parsed) || expected.HasValue && parsed != expected.Value)
        {
            throw Invalid(path, "整数字段不符合当前架构");
        }
    }

    private static void RequireInt64(JsonElement value, string path)
    {
        RequireKind(value, JsonValueKind.Number, path);
        if (!value.TryGetInt64(out _))
        {
            throw Invalid(path, "长整数字段不符合当前架构");
        }
    }

    private static void RequireDouble(JsonElement value, string path)
    {
        RequireKind(value, JsonValueKind.Number, path);
        if (!value.TryGetDouble(out double parsed) || double.IsNaN(parsed) || double.IsInfinity(parsed))
        {
            throw Invalid(path, "数值字段不符合当前架构");
        }
    }

    private static void RequireKind(JsonElement value, JsonValueKind expected, string path, JsonValueKind? alternate = null)
    {
        if (value.ValueKind != expected && value.ValueKind != alternate)
        {
            throw Invalid(path, $"字段类型应为 {expected}");
        }
    }

    private static InvalidDataException Invalid(string path, string reason) =>
        new($"{path}: {reason}");
}
