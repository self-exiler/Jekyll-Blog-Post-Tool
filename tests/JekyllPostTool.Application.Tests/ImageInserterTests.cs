using JekyllPostTool.Infrastructure.FileSystem;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// ImageInserter 单元测试，使用临时目录验证文件复制、冲突处理与 markdown 生成。
/// </summary>
public sealed class ImageInserterTests : IDisposable
{
    private readonly string _tempRoot = Path.Combine(Path.GetTempPath(), "ImageInserterTests-" + Guid.NewGuid().ToString("N"));
    private readonly BlogProject _project;
    private readonly ImageInserter _inserter = new();

    public ImageInserterTests()
    {
        Directory.CreateDirectory(_tempRoot);
        _project = new BlogProject(_tempRoot);
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempRoot, recursive: true); } catch { /* 忽略 */ }
    }

    [Fact]
    public async Task InsertAsync_CopiesImagesToTargetDirectory()
    {
        var srcA = CreateSourceImage("a.png");
        var srcB = CreateSourceImage("b.jpg");

        var markdown = await _inserter.InsertAsync(_project, "my-slug", new[] { srcA, srcB });

        Assert.True(File.Exists(Path.Combine(_project.Path, "assets", "img", "my-slug", "a.png")));
        Assert.True(File.Exists(Path.Combine(_project.Path, "assets", "img", "my-slug", "b.jpg")));
        Assert.Contains("![](/assets/img/my-slug/a.png)", markdown);
        Assert.Contains("![](/assets/img/my-slug/b.jpg)", markdown);
    }

    [Fact]
    public async Task InsertAsync_AppliesAltTextToAllImages()
    {
        var src = CreateSourceImage("photo.png");

        var markdown = await _inserter.InsertAsync(_project, "slug", new[] { src }, alt: "描述");

        Assert.Contains("![描述](/assets/img/slug/photo.png)", markdown);
    }

    [Fact]
    public async Task InsertAsync_ResolvesFileNameConflictWithSuffix()
    {
        var src = CreateSourceImage("dup.png");
        // 预先放置同名文件
        var targetDir = Path.Combine(_project.Path, "assets", "img", "slug");
        Directory.CreateDirectory(targetDir);
        await File.WriteAllTextAsync(Path.Combine(targetDir, "dup.png"), "existing");

        var markdown = await _inserter.InsertAsync(_project, "slug", new[] { src });

        Assert.True(File.Exists(Path.Combine(targetDir, "dup-1.png")));
        Assert.Contains("/assets/img/slug/dup-1.png", markdown);
        // 原文件内容未被覆盖
        Assert.Equal("existing", await File.ReadAllTextAsync(Path.Combine(targetDir, "dup.png")));
    }

    [Fact]
    public async Task InsertAsync_SkipsNonExistentSourceFiles()
    {
        var src = CreateSourceImage("real.png");
        var missing = Path.Combine(_tempRoot, "missing.png");

        var markdown = await _inserter.InsertAsync(_project, "slug", new[] { src, missing });

        Assert.Single(markdown.Split('\n', StringSplitOptions.RemoveEmptyEntries));
        Assert.Contains("real.png", markdown);
    }

    [Fact]
    public async Task InsertAsync_EmptySourceList_ReturnsEmptyMarkdown()
    {
        var markdown = await _inserter.InsertAsync(_project, "slug", Array.Empty<string>());

        Assert.Equal(string.Empty, markdown);
    }

    [Fact]
    public async Task InsertAsync_CreatesTargetDirectoryIfNotExists()
    {
        var src = CreateSourceImage("img.webp");

        await _inserter.InsertAsync(_project, "new-slug", new[] { src });

        Assert.True(Directory.Exists(Path.Combine(_project.Path, "assets", "img", "new-slug")));
    }

    [Fact]
    public async Task InsertAsync_EscapesAltTextMarkdownChars()
    {
        var src = CreateSourceImage("esc.png");

        // alt 含 markdown 链接语法字符时不得生成断裂的引用
        var markdown = await _inserter.InsertAsync(_project, "slug", new[] { src }, alt: "a[b](c)");

        Assert.Contains("![a\\[b\\]\\(c\\)](/assets/img/slug/esc.png)", markdown);
    }

    [Fact]
    public async Task InsertAsync_NullProject_Throws()
    {
        await Assert.ThrowsAsync<ArgumentNullException>(() =>
            _inserter.InsertAsync(null!, "slug", new[] { "x.png" }));
    }

    [Fact]
    public async Task InsertAsync_NullOrWhiteSpaceSlug_Throws()
    {
        await Assert.ThrowsAsync<ArgumentException>(() =>
            _inserter.InsertAsync(_project, "", new[] { "x.png" }));
        await Assert.ThrowsAsync<ArgumentException>(() =>
            _inserter.InsertAsync(_project, "   ", new[] { "x.png" }));
    }

    private string CreateSourceImage(string fileName)
    {
        var path = Path.Combine(_tempRoot, "source", fileName);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, "fake-image-content");
        return path;
    }
}
