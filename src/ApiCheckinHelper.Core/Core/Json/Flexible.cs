using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace ApiCheckinHelper.Core.Json;

public sealed class FlexibleIntConverter : JsonConverter<int>
{
    internal const int Unparsable = int.MinValue;

    public override int Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return ReadInt(ref reader);
    }

    private static int ReadInt(ref Utf8JsonReader reader)
    {
        return Coerce.ToInt(FlexibleIntConverter.ReadRaw(ref reader), Unparsable, int.MinValue, int.MaxValue);
    }

    public override void Write(Utf8JsonWriter writer, int value, JsonSerializerOptions options)
    {
        writer.WriteNumberValue(value);
    }

    internal static object? ReadRaw(ref Utf8JsonReader reader)
    {
        switch (reader.TokenType)
        {
            case JsonTokenType.String:
                return reader.GetString();
            case JsonTokenType.Number:
                if (reader.TryGetInt64(out long asLong))
                {
                    return asLong;
                }
                return reader.TryGetDouble(out double asDouble) ? asDouble : null;
            case JsonTokenType.True:
                return true;
            case JsonTokenType.False:
                return false;
            case JsonTokenType.Null:
                return null;
            default:
                reader.Skip();
                return null;
        }
    }
}

public sealed class FlexibleLongConverter : JsonConverter<long>
{
    internal const long Unparsable = long.MinValue;

    public override long Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return Coerce.ToLong(FlexibleIntConverter.ReadRaw(ref reader), Unparsable, long.MinValue, long.MaxValue);
    }

    public override void Write(Utf8JsonWriter writer, long value, JsonSerializerOptions options)
    {
        writer.WriteNumberValue(value);
    }
}

public sealed class FlexibleDoubleConverter : JsonConverter<double>
{
    internal const double Unparsable = double.MinValue;

    public override double Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        double parsed = Coerce.ToFloat(FlexibleIntConverter.ReadRaw(ref reader), Unparsable);
        return double.IsNaN(parsed) || double.IsInfinity(parsed) ? Unparsable : parsed;
    }

    public override void Write(Utf8JsonWriter writer, double value, JsonSerializerOptions options)
    {
        if (double.IsNaN(value) || double.IsInfinity(value))
        {
            writer.WriteNumberValue(0d);
            return;
        }
        writer.WriteNumberValue(value);
    }
}

public sealed class FlexibleBoolConverter : JsonConverter<bool>
{
    public override bool Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return Coerce.ToBool(FlexibleIntConverter.ReadRaw(ref reader));
    }

    public override void Write(Utf8JsonWriter writer, bool value, JsonSerializerOptions options)
    {
        writer.WriteBooleanValue(value);
    }
}

public sealed class FlexibleStringConverter : JsonConverter<string>
{
    public override string Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return Coerce.ToText(FlexibleIntConverter.ReadRaw(ref reader));
    }

    public override void Write(Utf8JsonWriter writer, string value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(value ?? "");
    }
}

public sealed class FlexibleBoolNullableConverter : JsonConverter<bool?>
{
    public override bool? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null)
        {
            return null;
        }
        object? raw = FlexibleIntConverter.ReadRaw(ref reader);
        if (raw is null)
        {
            return null;
        }
        if (raw is bool direct)
        {
            return direct;
        }
        if (raw is string text)
        {
            string trimmed = text.Trim();
            if (trimmed.Length == 0)
            {
                return null;
            }
            return bool.TryParse(trimmed, out bool parsed) ? parsed : null;
        }
        return Coerce.ToBool(raw);
    }

    public override void Write(Utf8JsonWriter writer, bool? value, JsonSerializerOptions options)
    {
        if (value.HasValue)
        {
            writer.WriteBooleanValue(value.Value);
            return;
        }
        writer.WriteNullValue();
    }
}

public static class Flexible
{
    public static JsonSerializerOptions Options { get; } = Build();

    private static JsonSerializerOptions Build()
    {
        JsonSerializerOptions options = new()
        {
            PropertyNameCaseInsensitive = true,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            WriteIndented = false,
            NumberHandling = JsonNumberHandling.AllowReadingFromString,
            ReadCommentHandling = JsonCommentHandling.Skip,
            AllowTrailingCommas = true,
        };
        options.Converters.Add(new FlexibleIntConverter());
        options.Converters.Add(new FlexibleLongConverter());
        options.Converters.Add(new FlexibleDoubleConverter());
        options.Converters.Add(new FlexibleBoolConverter());
        options.Converters.Add(new FlexibleStringConverter());
        options.Converters.Add(new FlexibleBoolNullableConverter());
        options.Converters.Add(new JsonStringEnumConverter());
        return options;
    }

    public static string Format(double value)
    {
        return value.ToString("0.####", CultureInfo.InvariantCulture);
    }
}
