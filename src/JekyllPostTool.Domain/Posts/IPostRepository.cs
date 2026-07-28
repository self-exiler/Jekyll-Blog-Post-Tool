namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 博文仓储接口。
/// </summary>
public interface IPostRepository
{
    Task<bool> ExistsAsync(string filePath, CancellationToken cancellationToken = default);

    Task<Post?> LoadAsync(string filePath, CancellationToken cancellationToken = default);

    Task SaveAsync(Post post, CancellationToken cancellationToken = default);

    Task<string> ReadBodyAsync(string filePath, CancellationToken cancellationToken = default);
}
