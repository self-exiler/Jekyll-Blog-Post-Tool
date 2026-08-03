using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// PostCreateUseCase 单元测试，使用 stub 仓储。
/// </summary>
public class PostCreateUseCaseTests
{
    private static readonly BlogProject Project = new(Path.GetTempPath());

    private sealed class StubPostRepository : IPostRepository
    {
        private readonly HashSet<string> _existingFiles = [];

        public void AddExistingFile(string path) => _existingFiles.Add(path);

        public bool Exists(string filePath) => _existingFiles.Contains(filePath);

        public void Delete(string filePath) => _existingFiles.Remove(filePath);

        public Task<Post?> LoadAsync(string filePath, CancellationToken ct = default)
            => Task.FromResult<Post?>(null);

        public Task SaveAsync(Post post, CancellationToken ct = default)
        {
            _existingFiles.Add(post.FilePath);
            return Task.CompletedTask;
        }

        public Task<string?> ReadAllTextAsync(string filePath, CancellationToken ct = default)
            => Task.FromResult<string?>(null);
    }

    private static FrontMatter ValidFrontMatter() => new()
    {
        Title = "Hello World",
        Date = new DateTimeOffset(2026, 7, 28, 0, 0, 0, TimeSpan.Zero),
        Authors = new[] { "cotes" }
    };

    [Fact]
    public async Task CreateAsync_ValidInput_SavesAndReturnsSuccess()
    {
        var postRepo = new StubPostRepository();
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var result = await useCase.CreateAsync(Project, ValidFrontMatter());

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
    }

    [Fact]
    public async Task CreateAsync_InvalidTitle_ReturnsValidationFailure()
    {
        var postRepo = new StubPostRepository();
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var fm = ValidFrontMatter();
        fm.Title = "";
        var result = await useCase.CreateAsync(Project, fm);

        Assert.False(result.IsSuccess);
        Assert.NotEmpty(result.Errors);
    }

    [Fact]
    public async Task CreateAsync_ConflictNoResolution_ReturnsConflict()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var result = await useCase.CreateAsync(Project, ValidFrontMatter());

        Assert.True(result.IsConflict);
        Assert.NotNull(result.Conflict);
    }

    [Fact]
    public async Task CreateAsync_ConflictWithAutoSuffix_SavesWithSuffix()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var result = await useCase.CreateAsync(Project, ValidFrontMatter(), conflictResolution: ConflictResolutionKind.AutoSuffix);

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world-1.md", result.FilePath);
    }

    [Fact]
    public async Task CreateAsync_ConflictWithOverwrite_SavesToOriginalPath()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var result = await useCase.CreateAsync(Project, ValidFrontMatter(), conflictResolution: ConflictResolutionKind.Overwrite);

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
    }

    [Fact]
    public async Task CreateAsync_NoConflict_SucceedsWithoutResolution()
    {
        var postRepo = new StubPostRepository();
        var authorRepo = new StubAuthorRepository(new[] { new Author("cotes", "Cotes") });
        var useCase = new PostCreateUseCase(postRepo, authorRepo, new FilenameConflictResolver(postRepo));

        var result = await useCase.CreateAsync(Project, ValidFrontMatter(), conflictResolution: ConflictResolutionKind.AutoSuffix);

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
    }
}

/// <summary>
/// 用于测试的 stub 仓储。
/// </summary>
file sealed class StubAuthorRepository : IAuthorRepository
{
    private readonly IReadOnlyList<Author> _authors;

    public StubAuthorRepository(IReadOnlyList<Author> authors) => _authors = authors;

    public Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken ct = default)
        => Task.FromResult(_authors);

    public Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken ct = default)
        => Task.CompletedTask;
}
