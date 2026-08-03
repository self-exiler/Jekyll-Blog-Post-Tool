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

    public bool Exists(string filePath) => File.Exists(filePath);

    public void Delete(string filePath)
    {
        if (File.Exists(filePath))
        {
            File.Delete(filePath);
        }
    }

    public async Task<Post?> LoadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(filePath))
        {
            return null;
        }

        var content = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
        MarkdownSplitter.TrySplit(content, out var frontMatterYaml, out var body);
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

        await File.WriteAllTextAsync(post.FilePath, content, OutputEncoding, cancellationToken);
    }

    public async Task<string?> ReadAllTextAsync(string filePath, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(filePath))
        {
            return null;
        }

        return await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
    }
}
