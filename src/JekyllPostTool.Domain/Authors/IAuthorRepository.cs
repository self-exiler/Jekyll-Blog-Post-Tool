namespace JekyllPostTool.Domain.Authors;

/// <summary>
/// 作者仓储接口。
/// </summary>
public interface IAuthorRepository
{
    Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken cancellationToken = default);

    Task<Author?> FindByIdAsync(string id, CancellationToken cancellationToken = default);

    Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken cancellationToken = default);
}
