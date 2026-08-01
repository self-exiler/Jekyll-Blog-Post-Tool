using JekyllPostTool.Domain.Authors;

namespace JekyllPostTool.Application.Authors;

/// <summary>
/// 作者增删改用例。冲突时抛 <see cref="InvalidOperationException"/>，由调用方捕获展示。
/// </summary>
public sealed class AuthorCrudUseCase
{
    private readonly IAuthorRepository _repository;

    public AuthorCrudUseCase(IAuthorRepository repository)
    {
        _repository = repository;
    }

    public async Task<IReadOnlyList<Author>> ListAsync(CancellationToken cancellationToken = default)
    {
        return await _repository.GetAllAsync(cancellationToken);
    }

    public async Task AddAsync(Author author, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();

        if (authors.Any(a => a.Id.Equals(author.Id, StringComparison.Ordinal)))
        {
            throw new InvalidOperationException($"作者 id '{author.Id}' 已存在");
        }

        authors.Add(author);
        await _repository.SaveAsync(authors, cancellationToken);
    }

    public async Task UpdateAsync(Author author, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();
        var index = authors.FindIndex(a => a.Id.Equals(author.Id, StringComparison.Ordinal));

        if (index < 0)
        {
            throw new InvalidOperationException($"作者 id '{author.Id}' 不存在");
        }

        authors[index] = author;
        await _repository.SaveAsync(authors, cancellationToken);
    }

    public async Task DeleteAsync(string id, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();
        var author = authors.FirstOrDefault(a => a.Id.Equals(id, StringComparison.Ordinal));

        if (author is null)
        {
            throw new InvalidOperationException($"作者 id '{id}' 不存在");
        }

        authors.Remove(author);
        await _repository.SaveAsync(authors, cancellationToken);
    }
}
