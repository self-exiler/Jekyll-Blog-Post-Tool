namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 将 markdown 文档切分为 front matter YAML 与正文段。
/// </summary>
public static class MarkdownSplitter
{
    /// <summary>
    /// 尝试切分。返回 false 表示无有效 front matter 块，<paramref name="body"/> 为原始内容。
    /// </summary>
    public static bool TrySplit(string content, out string frontMatterYaml, out string body)
    {
        frontMatterYaml = string.Empty;
        body = content;

        var trimmed = content.TrimStart();
        if (!trimmed.StartsWith("---", StringComparison.Ordinal))
        {
            return false;
        }

        var firstIndex = content.IndexOf("---", StringComparison.Ordinal);
        var secondIndex = content.IndexOf("---", firstIndex + 3, StringComparison.Ordinal);

        if (secondIndex < 0)
        {
            return false;
        }

        frontMatterYaml = content[(firstIndex + 3)..secondIndex].Trim();
        body = content[(secondIndex + 3)..].TrimStart('\r', '\n');
        return true;
    }
}
