namespace ApiCheckinHelper.Core;

public static class Tokens
{
    public static string MaskToken(string? token) => Mask(Auth.StripBearer(token), 4, 4);

    public static string Describe(Credential? credential)
    {
        if (credential is null || credential.Token.Length == 0)
        {
            return "未配置";
        }
        return $"令牌 {MaskToken(credential.Token)}";
    }

    private static string Mask(string value, int headKeep, int tailKeep)
    {
        if (value.Length == 0)
        {
            return "";
        }
        if (value.Length <= headKeep + tailKeep)
        {
            return new string('•', Math.Max(4, value.Length));
        }
        string prefix = headKeep > 0 ? value[..headKeep] : "";
        string suffix = tailKeep > 0 ? value[^tailKeep..] : "";
        int dots = Math.Min(16, Math.Max(4, value.Length - headKeep - tailKeep));
        return prefix + new string('•', dots) + suffix;
    }
}
