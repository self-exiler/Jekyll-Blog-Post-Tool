using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// FilenameConflictResolver 单元测试。
/// </summary>
public class FilenameConflictResolverTests
{
    private static readonly BlogProject Project = new(Path.GetTempPath());

    private sealed class StubPostRepository : IPostRepository
    {
        private readonly HashSet<string> _existingFiles = [];

        public void AddExistingFile(string path) => _existingFiles.Add(path);

        public bool Exists(string filePath) => _existingFiles.Contains(filePath);

        public Task<Post?> LoadAsync(string filePath, CancellationToken ct = default)
            => Task.FromResult<Post?>(null);

        public Task SaveAsync(Post post, CancellationToken ct = default)
            => Task.CompletedTask;

        public Task<string?> ReadAllTextAsync(string filePath, CancellationToken ct = default)
            => Task.FromResult<string?>(null);
    }

    [Fact]
    public void Check_NoExistingFile_ReturnsNoConflict()
    {
        var repo = new StubPostRepository();
        var resolver = new FilenameConflictResolver(repo);

        var result = resolver.Check(Project, "2026-07-28-test.md");

        Assert.False(result.HasConflict);
    }

    [Fact]
    public void Check_FileExists_ReturnsConflictWithTwoResolutions()
    {
        var repo = new StubPostRepository();
        repo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-test.md"));
        var resolver = new FilenameConflictResolver(repo);

        var result = resolver.Check(Project, "2026-07-28-test.md");

        Assert.True(result.HasConflict);
        Assert.Equal(2, result.Resolutions.Count);
    }

    [Fact]
    public void Check_AutoSuffixFindsNextAvailable()
    {
        var repo = new StubPostRepository();
        var dir = Project.PostsDirectory;
        repo.AddExistingFile(Path.Combine(dir, "2026-07-28-test.md"));
        repo.AddExistingFile(Path.Combine(dir, "2026-07-28-test-1.md"));
        var resolver = new FilenameConflictResolver(repo);

        var result = resolver.Check(Project, "2026-07-28-test.md");

        var autoSuffix = result.Resolutions.First(r => r.Kind == ConflictResolutionKind.AutoSuffix);
        Assert.Equal(2, autoSuffix.Suffix);
    }

    [Fact]
    public void ResolveFilePath_AutoSuffix_AppendsSuffix()
    {
        var repo = new StubPostRepository();
        repo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-test.md"));
        var resolver = new FilenameConflictResolver(repo);

        var conflict = resolver.Check(Project, "2026-07-28-test.md");
        var filePath = conflict.ResolveFilePath(ConflictResolutionKind.AutoSuffix);

        Assert.EndsWith("2026-07-28-test-1.md", filePath);
    }

    [Fact]
    public void ResolveFilePath_Overwrite_ReturnsOriginalPath()
    {
        var repo = new StubPostRepository();
        repo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-test.md"));
        var resolver = new FilenameConflictResolver(repo);

        var conflict = resolver.Check(Project, "2026-07-28-test.md");
        var filePath = conflict.ResolveFilePath(ConflictResolutionKind.Overwrite);

        Assert.EndsWith("2026-07-28-test.md", filePath);
    }

    [Fact]
    public void ResolveFilePath_NoConflict_ReturnsFilePath()
    {
        var repo = new StubPostRepository();
        var resolver = new FilenameConflictResolver(repo);

        var conflict = resolver.Check(Project, "2026-07-28-test.md");
        var filePath = conflict.ResolveFilePath(null);

        Assert.EndsWith("2026-07-28-test.md", filePath);
    }

    [Fact]
    public void ResolveFilePath_Null_Throws()
    {
        var repo = new StubPostRepository();
        repo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-test.md"));
        var resolver = new FilenameConflictResolver(repo);

        var conflict = resolver.Check(Project, "2026-07-28-test.md");

        Assert.Throws<InvalidOperationException>(() => conflict.ResolveFilePath(null));
    }
}
