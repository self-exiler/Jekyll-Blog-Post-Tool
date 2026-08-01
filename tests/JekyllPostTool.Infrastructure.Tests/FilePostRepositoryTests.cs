using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.FileSystem;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// FilePostRepository 集成测试，使用临时目录。
/// </summary>
public class FilePostRepositoryTests : IDisposable
{
    private readonly string _tempDir;

    public FilePostRepositoryTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"jpt-test-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDir))
        {
            Directory.Delete(_tempDir, true);
        }
    }

    [Fact]
    public void Exists_NonExistentFile_ReturnsFalse()
    {
        var repo = new FilePostRepository();
        Assert.False(repo.Exists(Path.Combine(_tempDir, "nope.md")));
    }

    [Fact]
    public void Exists_ExistingFile_ReturnsTrue()
    {
        var path = Path.Combine(_tempDir, "test.md");
        File.WriteAllText(path, "content");
        var repo = new FilePostRepository();
        Assert.True(repo.Exists(path));
    }

    [Fact]
    public async Task SaveAsync_CreatesFileWithFrontMatter()
    {
        var repo = new FilePostRepository();
        var fm = new FrontMatter
        {
            Title = "Test Post",
            Date = new DateTimeOffset(2026, 7, 28, 0, 0, 0, TimeSpan.Zero),
            Authors = new[] { "cotes" }
        };
        var post = new Post(Path.Combine(_tempDir, "post.md"), fm, "This is the body.");

        await repo.SaveAsync(post);

        Assert.True(File.Exists(post.FilePath));
        var content = await File.ReadAllTextAsync(post.FilePath);
        Assert.StartsWith("---", content);
        Assert.Contains("title: Test Post", content);
        Assert.Contains("This is the body.", content);
    }

    [Fact]
    public async Task SaveAsync_OverwriteExistingFile()
    {
        var repo = new FilePostRepository();
        var path = Path.Combine(_tempDir, "overwrite.md");
        var fm1 = new FrontMatter { Title = "Old", Date = DateTimeOffset.Now };
        await repo.SaveAsync(new Post(path, fm1, "old body"));

        var fm2 = new FrontMatter { Title = "New", Date = DateTimeOffset.Now };
        await repo.SaveAsync(new Post(path, fm2, "new body"));

        var loaded = await repo.LoadAsync(path);
        Assert.Equal("New", loaded!.FrontMatter.Title);
        Assert.Equal("new body", loaded.Body);
    }

    [Fact]
    public async Task LoadAsync_NonExistentFile_ReturnsNull()
    {
        var repo = new FilePostRepository();
        var result = await repo.LoadAsync(Path.Combine(_tempDir, "ghost.md"));
        Assert.Null(result);
    }

    [Fact]
    public async Task LoadAsync_ValidFile_ReturnsPostWithFrontMatterAndBody()
    {
        var path = Path.Combine(_tempDir, "load.md");
        var content = "---\ntitle: Loaded Post\ndate: 2026-07-28\nauthors:\n  - cotes\n---\nThis is the body text.";
        await File.WriteAllTextAsync(path, content);

        var repo = new FilePostRepository();
        var post = await repo.LoadAsync(path);

        Assert.NotNull(post);
        Assert.Equal("Loaded Post", post!.FrontMatter.Title);
        Assert.True(post.FrontMatter.Date.HasValue);
        Assert.Equal("This is the body text.", post.Body);
    }

    [Fact]
    public async Task SaveAsync_UsesLfLineEndings()
    {
        var repo = new FilePostRepository();
        var fm = new FrontMatter { Title = "LF Test", Date = DateTimeOffset.Now };
        var post = new Post(Path.Combine(_tempDir, "lf.md"), fm, "body");

        await repo.SaveAsync(post);

        var bytes = await File.ReadAllBytesAsync(post.FilePath);
        Assert.DoesNotContain((byte)'\r', bytes);
    }

    [Fact]
    public async Task SaveAsync_UsesUtf8WithoutBom()
    {
        var repo = new FilePostRepository();
        var fm = new FrontMatter { Title = "中文标题", Date = DateTimeOffset.Now };
        var post = new Post(Path.Combine(_tempDir, "utf8.md"), fm, "正文");

        await repo.SaveAsync(post);

        var bytes = await File.ReadAllBytesAsync(post.FilePath);
        Assert.False(bytes.Length >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF);
    }

    [Fact]
    public async Task ReadAllTextAsync_NonExistentFile_ReturnsNull()
    {
        var repo = new FilePostRepository();
        var result = await repo.ReadAllTextAsync(Path.Combine(_tempDir, "ghost.md"));
        Assert.Null(result);
    }

    [Fact]
    public async Task SaveThenLoad_RoundTripsCorrectly()
    {
        var repo = new FilePostRepository();
        var fm = new FrontMatter
        {
            Title = "中文 Round Trip",
            Date = new DateTimeOffset(2026, 7, 28, 14, 10, 0, TimeSpan.FromHours(8)),
            Categories = new[] { new Category("博客"), new Category("技术") },
            Tags = new[] { new Tag("Jekyll"), new Tag("工具") },
            Authors = new[] { "cotes" },
            Description = "这是一篇测试文章"
        };
        var path = Path.Combine(_tempDir, "roundtrip.md");
        var post = new Post(path, fm, "正文内容");
        await repo.SaveAsync(post);

        var loaded = await repo.LoadAsync(path);

        Assert.NotNull(loaded);
        Assert.Equal(fm.Title, loaded!.FrontMatter.Title);
        Assert.Equal(fm.Date, loaded.FrontMatter.Date);
        Assert.Equal(fm.Categories.Count, loaded.FrontMatter.Categories.Count);
        Assert.Equal(fm.Tags.Count, loaded.FrontMatter.Tags.Count);
        Assert.Equal("正文内容", loaded.Body);
    }
}
