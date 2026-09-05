using System.Text;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Posts;

namespace JekyllPostTool.Infrastructure.FileSystem;

/// <summary>
/// 基于文件系统的博文仓储实现：纯文件 IO，格式细节全部委托 <see cref="PostFileFormat"/>。
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

    public async Task<PostRead?> ReadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var content = await ReadAllTextAsync(filePath, cancellationToken);
        if (content is null)
        {
            return null;
        }

        var (frontMatter, body) = PostFileFormat.Parse(content);
        return new PostRead(filePath, content, frontMatter, body);
    }

    public async Task SaveAsync(Post post, CancellationToken cancellationToken = default)
    {
        var directory = Path.GetDirectoryName(post.FilePath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var content = PostFileFormat.Format(post.FrontMatter, post.Body);

        // 先写临时文件再原子替换：写一半崩溃不会损坏已有博文
        var tempPath = post.FilePath + ".tmp";
        try
        {
            await File.WriteAllTextAsync(tempPath, content, OutputEncoding, cancellationToken);
            File.Move(tempPath, post.FilePath, overwrite: true);
        }
        catch
        {
            TryDelete(tempPath);
            throw;
        }
    }

    public async Task<string?> ReadAllTextAsync(string filePath, CancellationToken cancellationToken = default)
    {
        if (!File.Exists(filePath))
        {
            return null;
        }

        return await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
    }

    private static void TryDelete(string path)
    {
        try { File.Delete(path); } catch { /* 清理失败时忽略 */ }
    }
}
