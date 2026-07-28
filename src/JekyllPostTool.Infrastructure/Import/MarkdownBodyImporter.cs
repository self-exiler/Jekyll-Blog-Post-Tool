using System.Text;

namespace JekyllPostTool.Infrastructure.Import;

/// <summary>
/// 从外部 markdown 文件导入正文，剥离其 front matter。
/// </summary>
public sealed class MarkdownBodyImporter
{
    public async Task<string> ImportAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var content = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);

        if (!content.TrimStart().StartsWith("---", StringComparison.Ordinal))
        {
            return content;
        }

        var firstIndex = content.IndexOf("---", StringComparison.Ordinal);
        if (firstIndex < 0)
        {
            return content;
        }

        var secondIndex = content.IndexOf("---", firstIndex + 3, StringComparison.Ordinal);
        if (secondIndex < 0)
        {
            return content;
        }

        var body = content[(secondIndex + 3)..];
        return body.TrimStart('\r', '\n');
    }
}
