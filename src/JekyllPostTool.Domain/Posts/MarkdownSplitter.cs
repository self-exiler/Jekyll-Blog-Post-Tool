namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 将 markdown 文档切分为 front matter YAML 与正文段。
/// </summary>
public static class MarkdownSplitter
{
    private const string Delimiter = "---";

    /// <summary>
    /// 尝试切分。返回 false 表示无有效 front matter 块，<paramref name="body"/> 为原始内容。
    /// 仅在行首匹配 <c>---</c> 作为定界符，避免正文中含 <c>---</c>（如水平分割线）时误切分。
    /// </summary>
    public static bool TrySplit(string content, out string frontMatterYaml, out string body)
    {
        frontMatterYaml = string.Empty;
        body = content;

        if (string.IsNullOrEmpty(content))
        {
            return false;
        }

        var lines = content.Split('\n');

        // 查找开定界符：跳过前导空行，第一个非空行必须是 "---"
        var openIndex = -1;
        for (var i = 0; i < lines.Length; i++)
        {
            var trimmed = lines[i].Trim('\r', ' ', '\t');
            if (trimmed.Length == 0)
            {
                continue;
            }

            if (trimmed == Delimiter)
            {
                openIndex = i;
            }

            break;
        }

        if (openIndex < 0)
        {
            return false;
        }

        // 查找闭定界符：后续第一个独占一行的 "---"
        var closeIndex = -1;
        for (var i = openIndex + 1; i < lines.Length; i++)
        {
            if (lines[i].Trim('\r', ' ', '\t') == Delimiter)
            {
                closeIndex = i;
                break;
            }
        }

        if (closeIndex < 0)
        {
            return false;
        }

        // 提取 YAML（开闭定界符之间的行）
        frontMatterYaml = JoinLines(lines, openIndex + 1, closeIndex).Trim();

        // 提取正文（闭定界符之后），去除前导换行
        body = JoinLines(lines, closeIndex + 1, lines.Length).TrimStart('\r', '\n');

        return true;
    }

    private static string JoinLines(string[] lines, int start, int end) =>
        start >= end ? string.Empty : string.Join('\n', lines[start..end]);
}
