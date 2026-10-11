namespace ApiCheckinHelper.Core;

public static class Coerce
{
    private static readonly HashSet<string> TrueWords =
        new(StringComparer.OrdinalIgnoreCase) { "1", "true", "yes", "on", "y", "t", "是", "开", "启用" };

    private static readonly HashSet<string> FalseWords =
        new(StringComparer.OrdinalIgnoreCase) { "0", "false", "no", "off", "n", "f", "否", "关", "禁用", "" };

    public static int ToInt(object? value, int fallback = 0, int? low = null, int? high = null)
    {
        long number;
        switch (value)
        {
            case null:
                number = fallback;
                break;
            case bool flag:
                number = flag ? 1 : 0;
                break;
            case int intValue:
                number = intValue;
                break;
            case long longValue:
                number = longValue;
                break;
            case double doubleValue when !double.IsNaN(doubleValue) && !double.IsInfinity(doubleValue):
                number = (long)Math.Round(doubleValue, MidpointRounding.AwayFromZero);
                break;
            case float floatValue when !float.IsNaN(floatValue) && !float.IsInfinity(floatValue):
                number = (long)Math.Round(floatValue, MidpointRounding.AwayFromZero);
                break;
            case decimal decimalValue:
                number = (long)Math.Round(decimalValue, MidpointRounding.AwayFromZero);
                break;
            case string text:
                number = ParseText(text, fallback);
                break;
            default:
                number = fallback;
                break;
        }
        if (number > int.MaxValue)
        {
            number = int.MaxValue;
        }
        if (number < int.MinValue)
        {
            number = int.MinValue;
        }
        if (low.HasValue && number < low.Value)
        {
            return low.Value;
        }
        if (high.HasValue && number > high.Value)
        {
            return high.Value;
        }
        return (int)number;
    }

    private static long ParseText(string text, int fallback)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0)
        {
            return fallback;
        }
        if (long.TryParse(trimmed, System.Globalization.NumberStyles.Integer, System.Globalization.CultureInfo.InvariantCulture, out long direct))
        {
            return direct;
        }
        if (double.TryParse(trimmed, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double parsed))
        {
            if (double.IsNaN(parsed) || double.IsInfinity(parsed))
            {
                return fallback;
            }
            return (long)Math.Round(parsed, MidpointRounding.AwayFromZero);
        }
        return fallback;
    }

    public static long ToLong(object? value, long fallback = 0L, long? low = null, long? high = null)
    {
        long number;
        switch (value)
        {
            case null:
                number = fallback;
                break;
            case bool flag:
                number = flag ? 1L : 0L;
                break;
            case int intValue:
                number = intValue;
                break;
            case long longValue:
                number = longValue;
                break;
            case double doubleValue when !double.IsNaN(doubleValue) && !double.IsInfinity(doubleValue):
                number = (long)Math.Round(doubleValue, MidpointRounding.AwayFromZero);
                break;
            case float floatValue when !float.IsNaN(floatValue) && !float.IsInfinity(floatValue):
                number = (long)Math.Round(floatValue, MidpointRounding.AwayFromZero);
                break;
            case decimal decimalValue:
                number = (long)Math.Round(decimalValue, MidpointRounding.AwayFromZero);
                break;
            case string text:
                number = ParseLong(text, fallback);
                break;
            default:
                number = fallback;
                break;
        }
        if (low.HasValue && number < low.Value)
        {
            return low.Value;
        }
        if (high.HasValue && number > high.Value)
        {
            return high.Value;
        }
        return number;
    }

    private static long ParseLong(string text, long fallback)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0)
        {
            return fallback;
        }
        if (long.TryParse(trimmed, System.Globalization.NumberStyles.Integer, System.Globalization.CultureInfo.InvariantCulture, out long direct))
        {
            return direct;
        }
        if (double.TryParse(trimmed, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double parsed))
        {
            if (double.IsNaN(parsed) || double.IsInfinity(parsed))
            {
                return fallback;
            }
            return (long)Math.Round(parsed, MidpointRounding.AwayFromZero);
        }
        return fallback;
    }

    public static double ToFloat(object? value, double fallback = 0d, double? low = null, double? high = null)
    {
        double number;
        switch (value)
        {
            case bool flag:
                number = flag ? 1d : 0d;
                break;
            case int intValue:
                number = intValue;
                break;
            case long longValue:
                number = longValue;
                break;
            case double doubleValue:
                number = doubleValue;
                break;
            case float floatValue:
                number = floatValue;
                break;
            case decimal decimalValue:
                number = (double)decimalValue;
                break;
            case string text:
                number = ParseFloat(text, fallback);
                break;
            default:
                number = fallback;
                break;
        }
        if (double.IsNaN(number) || double.IsInfinity(number))
        {
            number = fallback;
        }
        if (low.HasValue && number < low.Value)
        {
            return low.Value;
        }
        if (high.HasValue && number > high.Value)
        {
            return high.Value;
        }
        return number;
    }

    private static double ParseFloat(string text, double fallback)
    {
        string trimmed = text.Trim();
        if (trimmed.Length == 0)
        {
            return fallback;
        }
        if (double.TryParse(trimmed, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out double parsed))
        {
            return parsed;
        }
        return fallback;
    }

    public static bool ToBool(object? value, bool fallback = false)
    {
        switch (value)
        {
            case bool flag:
                return flag;
            case int intValue:
                return intValue != 0;
            case long longValue:
                return longValue != 0;
            case double doubleValue:
                return Math.Abs(doubleValue) > double.Epsilon;
            case string text:
            {
                string trimmed = text.Trim();
                if (TrueWords.Contains(trimmed))
                {
                    return true;
                }
                if (FalseWords.Contains(trimmed))
                {
                    return false;
                }
                return fallback;
            }
            default:
                return fallback;
        }
    }

    public static string ToText(object? value, string fallback = "")
    {
        return value switch
        {
            null => fallback,
            string text => text,
            bool flag => flag ? "true" : "false",
            IFormattable formattable => formattable.ToString(null, System.Globalization.CultureInfo.InvariantCulture),
            _ => value.ToString() ?? fallback,
        };
    }
}
