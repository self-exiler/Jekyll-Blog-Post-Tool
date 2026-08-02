namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 正文插入操作的纯函数：在指定位置插入文本，或追加到末尾。
/// </summary>
public static class BodyInsertion
{
    /// <summary>
    /// 在光标位置插入 markdown；insertionIndex 为 null、负数或超出长度时追加到末尾。
    /// 行中插入时前后补换行，避免与现有文字粘连。
    /// </summary>
    public static string InsertAtCursor(string body, string markdown, int? insertionIndex)
    {
        markdown = markdown.TrimEnd();

        if (string.IsNullOrEmpty(body))
        {
            return markdown;
        }

        if (insertionIndex is null or < 0 || insertionIndex > body.Length)
        {
            return Append(body, markdown);
        }

        var prefix = body[..insertionIndex.Value];
        var suffix = body[insertionIndex.Value..];
        var needLeadingNewline = prefix.Length > 0 && !prefix.EndsWith('\n') && !prefix.EndsWith('\r');
        var needTrailingNewline = suffix.Length > 0 && !suffix.StartsWith('\n') && !suffix.StartsWith('\r');
        return prefix
            + (needLeadingNewline ? Environment.NewLine : string.Empty)
            + markdown
            + (needTrailingNewline ? Environment.NewLine : string.Empty)
            + suffix;
    }

    /// <summary>
    /// 追加到末尾（非空时以换行分隔）。
    /// </summary>
    private static string Append(string body, string text)
    {
        text = text.TrimEnd();
        return string.IsNullOrEmpty(body) ? text : body + Environment.NewLine + text;
    }
}
