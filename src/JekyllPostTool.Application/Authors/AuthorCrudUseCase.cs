using JekyllPostTool.Domain.Authors;

namespace JekyllPostTool.Application.Authors;

/// <summary>
/// 作者增删改用例。
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

    public async Task<Author?> GetAsync(string id, CancellationToken cancellationToken = default)
    {
        return await _repository.FindByIdAsync(id, cancellationToken);
    }

    public async Task<AuthorOperationResult> AddAsync(Author author, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();

        if (authors.Any(a => a.Id.Equals(author.Id, StringComparison.Ordinal)))
        {
            return AuthorOperationResult.Failure($"作者 id '{author.Id}' 已存在");
        }

        authors.Add(author);
        await _repository.SaveAsync(authors, cancellationToken);
        return AuthorOperationResult.Success();
    }

    public async Task<AuthorOperationResult> UpdateAsync(Author author, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();
        var index = authors.FindIndex(a => a.Id.Equals(author.Id, StringComparison.Ordinal));

        if (index < 0)
        {
            return AuthorOperationResult.Failure($"作者 id '{author.Id}' 不存在");
        }

        authors[index] = author;
        await _repository.SaveAsync(authors, cancellationToken);
        return AuthorOperationResult.Success();
    }

    public async Task<AuthorOperationResult> DeleteAsync(string id, CancellationToken cancellationToken = default)
    {
        var authors = (await _repository.GetAllAsync(cancellationToken)).ToList();
        var author = authors.FirstOrDefault(a => a.Id.Equals(id, StringComparison.Ordinal));

        if (author is null)
        {
            return AuthorOperationResult.Failure($"作者 id '{id}' 不存在");
        }

        authors.Remove(author);
        await _repository.SaveAsync(authors, cancellationToken);
        return AuthorOperationResult.Success();
    }
}

public sealed class AuthorOperationResult
{
    public bool IsSuccess { get; }

    public string? Error { get; }

    private AuthorOperationResult(bool isSuccess, string? error)
    {
        IsSuccess = isSuccess;
        Error = error;
    }

    public static AuthorOperationResult Success() => new(true, null);

    public static AuthorOperationResult Failure(string error) => new(false, error);
}
