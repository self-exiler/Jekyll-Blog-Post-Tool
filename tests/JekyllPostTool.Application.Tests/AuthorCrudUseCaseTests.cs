using JekyllPostTool.Application.Authors;
using JekyllPostTool.Domain.Authors;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// AuthorCrudUseCase 单元测试，使用 stub 仓储。
/// </summary>
public class AuthorCrudUseCaseTests
{
    private sealed class StubAuthorRepository : IAuthorRepository
    {
        public List<Author> Authors { get; set; } = [];

        public Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken ct = default)
            => Task.FromResult<IReadOnlyList<Author>>(Authors);

        public Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken ct = default)
        {
            Authors = authors.ToList();
            return Task.CompletedTask;
        }
    }

    [Fact]
    public async Task AddAsync_NewId_Succeeds()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Cotes")] };
        var useCase = new AuthorCrudUseCase(repo);

        await useCase.AddAsync(new Author("admin", "Admin"));

        Assert.Contains(repo.Authors, a => a.Id == "admin");
    }

    [Fact]
    public async Task AddAsync_DuplicateId_Throws()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Cotes")] };
        var useCase = new AuthorCrudUseCase(repo);

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => useCase.AddAsync(new Author("cotes", "Another")));

        Assert.Contains("已存在", ex.Message);
    }

    [Fact]
    public async Task UpdateAsync_ExistingId_ReplacesAuthor()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Old Name")] };
        var useCase = new AuthorCrudUseCase(repo);

        await useCase.UpdateAsync(new Author("cotes", "New Name"));

        var updated = await repo.GetAllAsync();
        Assert.Equal("New Name", updated.First(a => a.Id == "cotes").Name);
    }

    [Fact]
    public async Task UpdateAsync_NonExistentId_Throws()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Cotes")] };
        var useCase = new AuthorCrudUseCase(repo);

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => useCase.UpdateAsync(new Author("ghost", "Ghost")));

        Assert.Contains("不存在", ex.Message);
    }

    [Fact]
    public async Task DeleteAsync_ExistingId_RemovesAuthor()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Cotes"), new Author("admin", "Admin")] };
        var useCase = new AuthorCrudUseCase(repo);

        await useCase.DeleteAsync("cotes");

        var remaining = await repo.GetAllAsync();
        Assert.Single(remaining);
        Assert.Equal("admin", remaining[0].Id);
    }

    [Fact]
    public async Task DeleteAsync_NonExistentId_Throws()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("cotes", "Cotes")] };
        var useCase = new AuthorCrudUseCase(repo);

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => useCase.DeleteAsync("ghost"));

        Assert.Contains("不存在", ex.Message);
    }

    [Fact]
    public async Task ListAsync_ReturnsAllAuthors()
    {
        var repo = new StubAuthorRepository { Authors = [new Author("a", "A"), new Author("b", "B")] };
        var useCase = new AuthorCrudUseCase(repo);

        var list = await useCase.ListAsync();

        Assert.Equal(2, list.Count);
    }
}
