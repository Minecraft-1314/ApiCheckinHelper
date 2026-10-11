using System.Text.Json;

namespace ApiCheckinHelper.Ui.Bridge;

public static class BridgeJson
{
    public static JsonSerializerOptions Options { get; } = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DictionaryKeyPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
    };

    public static string Serialize(object payload) => JsonSerializer.Serialize(payload, Options);

    public static string Text(JsonElement source, string name, string fallback = "")
    {
        return source.ValueKind == JsonValueKind.Object
            && source.TryGetProperty(name, out JsonElement value)
            && value.ValueKind == JsonValueKind.String
                ? value.GetString() ?? fallback
                : fallback;
    }

    public static int Int(JsonElement source, string name, int fallback = 0)
    {
        if (source.ValueKind != JsonValueKind.Object || !source.TryGetProperty(name, out JsonElement value))
        {
            return fallback;
        }
        return value.ValueKind switch
        {
            JsonValueKind.Number when value.TryGetInt32(out int number) => number,
            JsonValueKind.String when int.TryParse(value.GetString(), out int number) => number,
            _ => fallback,
        };
    }

    public static bool Bool(JsonElement source, string name, bool fallback = false)
    {
        if (source.ValueKind != JsonValueKind.Object || !source.TryGetProperty(name, out JsonElement value))
        {
            return fallback;
        }
        return value.ValueKind switch
        {
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.String when bool.TryParse(value.GetString(), out bool parsed) => parsed,
            _ => fallback,
        };
    }
}
