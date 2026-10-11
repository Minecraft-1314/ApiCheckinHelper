using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace ApiCheckinHelper.Core.Json;

public static class J
{
    public static JsonNode? Get(JsonNode? node, params string[] path)
    {
        JsonNode? current = node;
        foreach (string key in path)
        {
            if (current is not JsonObject map || !map.TryGetPropertyValue(key, out JsonNode? next))
            {
                return null;
            }
            current = next;
        }
        return current;
    }

    public static object? Raw(JsonNode? node)
    {
        return node switch
        {
            null => null,
            JsonObject or JsonArray => node.ToJsonString(),
            JsonValue value => Unwrap(value),
            _ => null,
        };
    }

    private static object? Unwrap(JsonValue value)
    {
        if (value.TryGetValue<bool>(out bool flag))
        {
            return flag;
        }
        if (value.TryGetValue<string>(out string? text))
        {
            return text;
        }
        if (value.TryGetValue<long>(out long integer))
        {
            return integer;
        }
        if (value.TryGetValue<double>(out double number))
        {
            return number;
        }
        if (value.TryGetValue<decimal>(out decimal money))
        {
            return (double)money;
        }
        try
        {
            return value.GetValue<object?>();
        }
        catch (InvalidOperationException)
        {
            return null;
        }
        catch (FormatException)
        {
            return null;
        }
    }

    public static string Str(JsonNode? node, params string[] path)
    {
        return Coerce.ToText(Get(node, path));
    }

    public static bool Bool(JsonNode? node, params string[] path)
    {
        return Coerce.ToBool(Raw(Get(node, path)));
    }

    public static bool? BoolOrNull(JsonNode? node, params string[] path)
    {
        object? raw = Raw(Get(node, path));
        return raw switch
        {
            null => null,
            bool flag => flag,
            string text => bool.TryParse(text.Trim(), out bool parsed) ? parsed : null,
            _ => Coerce.ToBool(raw),
        };
    }

    public static int Int(JsonNode? node, int fallback = 0, int low = int.MinValue, int high = int.MaxValue)
    {
        return Coerce.ToInt(Raw(Get(node)), fallback, low, high);
    }

    public static int Int(JsonNode? node, string key, int fallback, int low, int high)
    {
        return Coerce.ToInt(Raw(Get(node, key)), fallback, low, high);
    }

    public static long Long(JsonNode? node, long fallback = 0)
    {
        return Coerce.ToLong(Raw(Get(node)), fallback);
    }

    public static long Long(JsonNode? node, string key, long fallback)
    {
        return Coerce.ToLong(Raw(Get(node, key)), fallback);
    }

    public static double Dbl(JsonNode? node, double fallback = 0d, double low = double.MinValue, double high = double.MaxValue)
    {
        return Coerce.ToFloat(Raw(Get(node)), fallback, low, high);
    }

    public static double Dbl(JsonNode? node, string key, double fallback, double low, double high)
    {
        return Coerce.ToFloat(Raw(Get(node, key)), fallback, low, high);
    }

    public static JsonObject? Obj(JsonNode? node, params string[] path)
    {
        return Get(node, path) as JsonObject;
    }

    public static string Number(double value)
    {
        return value.ToString("0.####", CultureInfo.InvariantCulture);
    }
}
