using System.Text;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Infrastructure.Import;

/// <summary>
/// 从外部 markdown 文件导入正文，剥离其 front matter。
/// </summary>
public sealed class MarkdownBodyImporter
{
    public async Task<string> ImportAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var content = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
        MarkdownSplitter.TrySplit(content, out _, out var body);
        return body;
    }
}
