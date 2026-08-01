using JekyllPostTool.Infrastructure.Import;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// MarkdownBodyImporter 导入测试。
/// </summary>
public class MarkdownBodyImporterTests : IDisposable
{
    private readonly string _tempDir;

    public MarkdownBodyImporterTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"jpt-import-{Guid.NewGuid():N}");
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
    public async Task ImportAsync_FileWithFrontMatter_StripsFrontMatter()
    {
        var path = Path.Combine(_tempDir, "import.md");
        var content = "---\ntitle: Source Post\ndate: 2026-07-28\n---\nImported body text.";
        await File.WriteAllTextAsync(path, content);

        var importer = new MarkdownBodyImporter();
        var body = await importer.ImportAsync(path);

        Assert.Equal("Imported body text.", body);
    }

    [Fact]
    public async Task ImportAsync_FileWithoutFrontMatter_ReturnsAllContent()
    {
        var path = Path.Combine(_tempDir, "no-fm.md");
        await File.WriteAllTextAsync(path, "Just some content without front matter.");

        var importer = new MarkdownBodyImporter();
        var body = await importer.ImportAsync(path);

        Assert.Equal("Just some content without front matter.", body);
    }

    [Fact]
    public async Task ImportAsync_EmptyFile_ReturnsEmptyString()
    {
        var path = Path.Combine(_tempDir, "empty.md");
        await File.WriteAllTextAsync(path, "");

        var importer = new MarkdownBodyImporter();
        var body = await importer.ImportAsync(path);

        Assert.Equal(string.Empty, body);
    }

    [Fact]
    public async Task ImportAsync_FrontMatterOnly_ReturnsEmptyBody()
    {
        var path = Path.Combine(_tempDir, "fm-only.md");
        var content = "---\ntitle: Only FM\n---\n";
        await File.WriteAllTextAsync(path, content);

        var importer = new MarkdownBodyImporter();
        var body = await importer.ImportAsync(path);

        Assert.Equal(string.Empty, body);
    }
}
