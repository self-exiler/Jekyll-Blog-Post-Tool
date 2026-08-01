using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Infrastructure.FileSystem;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// YamlAuthorRepository 集成测试，使用临时文件。
/// </summary>
public class YamlAuthorRepositoryTests : IDisposable
{
    private readonly string _tempDir;
    private readonly string _authorsFile;

    public YamlAuthorRepositoryTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"jpt-authors-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);
        _authorsFile = Path.Combine(_tempDir, "authors.yml");
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDir))
        {
            Directory.Delete(_tempDir, true);
        }
    }

    [Fact]
    public async Task GetAllAsync_NonExistentFile_ReturnsEmpty()
    {
        var repo = new YamlAuthorRepository(() => _authorsFile);
        var authors = await repo.GetAllAsync();
        Assert.Empty(authors);
    }

    [Fact]
    public async Task SaveAsync_ThenGetAllAsync_RoundTripsAuthors()
    {
        var repo = new YamlAuthorRepository(() => _authorsFile);
        var authors = new List<Author>
        {
            new("cotes", "Cotes", "@cotes", "https://example.com"),
            new("admin", "Admin")
        };

        await repo.SaveAsync(authors);
        var loaded = await repo.GetAllAsync();

        Assert.Equal(2, loaded.Count);
        Assert.Equal("cotes", loaded[0].Id);
        Assert.Equal("Cotes", loaded[0].Name);
        Assert.Equal("@cotes", loaded[0].Twitter);
        Assert.Equal("https://example.com", loaded[0].Url);
        Assert.Equal("admin", loaded[1].Id);
        Assert.Null(loaded[1].Twitter);
        Assert.Null(loaded[1].Url);
    }

    [Fact]
    public async Task GetAllAsync_EmptyFile_ReturnsEmpty()
    {
        await File.WriteAllTextAsync(_authorsFile, "");

        var repo = new YamlAuthorRepository(() => _authorsFile);
        var authors = await repo.GetAllAsync();

        Assert.Empty(authors);
    }

    [Fact]
    public async Task GetAllAsync_ValidYaml_ParsesAllFields()
    {
        var yaml = """
cotes:
  name: Cotes
  twitter: "@cotes"
  url: https://cotes.page
admin:
  name: Admin
""";
        await File.WriteAllTextAsync(_authorsFile, yaml);

        var repo = new YamlAuthorRepository(() => _authorsFile);
        var authors = await repo.GetAllAsync();

        Assert.Equal(2, authors.Count);
        var cotes = authors.First(a => a.Id == "cotes");
        Assert.Equal("Cotes", cotes.Name);
        Assert.Equal("@cotes", cotes.Twitter);
        Assert.Equal("https://cotes.page", cotes.Url);
        var admin = authors.First(a => a.Id == "admin");
        Assert.Equal("Admin", admin.Name);
        Assert.Null(admin.Twitter);
        Assert.Null(admin.Url);
    }

    [Fact]
    public async Task SaveAsync_PreservesSpecialCharacters()
    {
        var repo = new YamlAuthorRepository(() => _authorsFile);
        var authors = new List<Author>
        {
            new("中文", "中文名", "@中文推特", "https://中文.url")
        };

        await repo.SaveAsync(authors);
        var loaded = await repo.GetAllAsync();

        Assert.Single(loaded);
        Assert.Equal("中文", loaded[0].Id);
        Assert.Equal("中文名", loaded[0].Name);
    }
}
