using System.Globalization;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 时区偏移量格式化（±hh:mm）与候选列表生成。无状态纯函数。
/// </summary>
public static class TimeZoneFormatter
{
    /// <summary>
    /// 构建 -12:00 到 +14:00（步长 30 分钟）的时区候选列表。
    /// </summary>
    public static IReadOnlyList<string> BuildOptions()
    {
        var options = new List<string>();
        for (var minutes = -12 * 60; minutes <= 14 * 60; minutes += 30)
        {
            options.Add(Format(TimeSpan.FromMinutes(minutes)));
        }

        return options;
    }

    /// <summary>
    /// 把偏移量格式化为 ±hh:mm。
    /// </summary>
    public static string Format(TimeSpan offset)
    {
        var sign = offset >= TimeSpan.Zero ? "+" : "-";
        var absolute = offset.Duration();
        return $"{sign}{absolute.Hours:D2}:{absolute.Minutes:D2}";
    }

    /// <summary>
    /// 解析 ±hh:mm；失败时回退到本地时区偏移。
    /// </summary>
    public static TimeSpan ParseOrLocal(string? text)
    {
        if (string.IsNullOrWhiteSpace(text) || text.Length < 6)
        {
            return DateTimeOffset.Now.Offset;
        }

        var sign = text[0];
        if (sign is not '+' and not '-')
        {
            return DateTimeOffset.Now.Offset;
        }

        if (TimeSpan.TryParseExact(text[1..], @"hh\:mm", CultureInfo.InvariantCulture, out var absolute))
        {
            return sign == '-' ? -absolute : absolute;
        }

        return DateTimeOffset.Now.Offset;
    }
}
