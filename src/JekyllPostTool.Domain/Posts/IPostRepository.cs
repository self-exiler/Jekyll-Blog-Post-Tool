namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 博文仓储接口。
/// </summary>
public interface IPostRepository
{
    bool Exists(string filePath);

    void Delete(string filePath);

    Task<Post?> LoadAsync(string filePath, CancellationToken cancellationToken = default);

    Task SaveAsync(Post post, CancellationToken cancellationToken = default);

    Task<string?> ReadAllTextAsync(string filePath, CancellationToken cancellationToken = default);
}
