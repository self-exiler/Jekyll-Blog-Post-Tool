using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Infrastructure.FileSystem;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 基于当前项目上下文的作者仓储适配器。
/// </summary>
public sealed class CurrentProjectAuthorRepository : IAuthorRepository
{
    private readonly IProjectContext _projectContext;

    public CurrentProjectAuthorRepository(IProjectContext projectContext)
    {
        _projectContext = projectContext;
    }

    public Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        var path = _projectContext.CurrentProject?.AuthorsFilePath;
        if (string.IsNullOrWhiteSpace(path))
        {
            return Task.FromResult<IReadOnlyList<Author>>(Array.Empty<Author>());
        }

        return new YamlAuthorRepository(path).GetAllAsync(cancellationToken);
    }

    public async Task<Author?> FindByIdAsync(string id, CancellationToken cancellationToken = default)
    {
        var authors = await GetAllAsync(cancellationToken);
        return authors.FirstOrDefault(a => a.Id.Equals(id, StringComparison.Ordinal));
    }

    public Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken cancellationToken = default)
    {
        var path = _projectContext.CurrentProject?.AuthorsFilePath;
        if (string.IsNullOrWhiteSpace(path))
        {
            throw new InvalidOperationException("未选择博客项目，无法保存作者信息。");
        }

        return new YamlAuthorRepository(path).SaveAsync(authors, cancellationToken);
    }
}
