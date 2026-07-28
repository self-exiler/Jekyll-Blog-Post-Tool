using System.Text;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Yaml;

namespace JekyllPostTool.Infrastructure.FileSystem;

/// <summary>
/// 基于文件系统的博文仓储实现。
/// </summary>
public sealed class FilePostRepository : IPostRepository
{
    private static readonly Encoding OutputEncoding = new UTF8Encoding(false);

    public Task<bool> ExistsAsync(string filePath, CancellationToken cancellationToken = default)
    {
        return Task.FromResult(File.Exists(filePath));
    }

    public async Task<Post?> LoadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(filePath))
        {
            return null;
        }

        var content = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
        var (frontMatterYaml, body) = SplitContent(content);
        var frontMatter = YamlFrontMatterParser.Parse(frontMatterYaml);

        return new Post(filePath, frontMatter, body);
    }

    public async Task SaveAsync(Post post, CancellationToken cancellationToken = default)
    {
        var directory = Path.GetDirectoryName(post.FilePath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var frontMatterYaml = YamlFrontMatterSerializer.Serialize(post.FrontMatter);
        var content = $"---{Environment.NewLine}{frontMatterYaml}{Environment.NewLine}---{Environment.NewLine}{post.Body}";

        // 统一转换为 LF
        content = content.Replace("\r\n", "\n");

        var tempFilePath = Path.GetTempFileName();
        await File.WriteAllTextAsync(tempFilePath, content, OutputEncoding, cancellationToken);

        try
        {
            if (File.Exists(post.FilePath))
            {
                File.Replace(tempFilePath, post.FilePath, null);
            }
            else
            {
                File.Move(tempFilePath, post.FilePath);
            }
        }
        catch
        {
            if (File.Exists(tempFilePath))
            {
                File.Delete(tempFilePath);
            }

            throw;
        }
    }

    public async Task<string> ReadBodyAsync(string filePath, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(filePath))
        {
            return string.Empty;
        }

        var content = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
        var (_, body) = SplitContent(content);
        return body;
    }

    private static (string FrontMatter, string Body) SplitContent(string content)
    {
        var trimmed = content.TrimStart();

        if (!trimmed.StartsWith("---", StringComparison.Ordinal))
        {
            return (string.Empty, content);
        }

        var firstIndex = content.IndexOf("---", StringComparison.Ordinal);
        var secondIndex = content.IndexOf("---", firstIndex + 3, StringComparison.Ordinal);

        if (secondIndex < 0)
        {
            return (string.Empty, content);
        }

        var frontMatter = content[(firstIndex + 3)..secondIndex].Trim();
        var body = content[(secondIndex + 3)..].TrimStart('\r', '\n');

        return (frontMatter, body);
    }
}
