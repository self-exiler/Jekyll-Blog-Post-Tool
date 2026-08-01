using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Domain.Tests;

/// <summary>
/// Post 实体与文件名生成测试。
/// </summary>
public class PostTests
{
    [Fact]
    public void BuildFileName_GeneratesCorrectFormat()
    {
        var date = new DateTimeOffset(2026, 7, 28, 14, 10, 0, TimeSpan.FromHours(8));
        var slug = new Slug("hello-world");

        var fileName = Post.BuildFileName(date, slug);

        Assert.Equal("2026-07-28-hello-world.md", fileName);
    }

    [Fact]
    public void BuildFileName_ChineseSlug_PreservedInFileName()
    {
        var date = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.FromHours(8));
        var slug = new Slug("中文标题");

        Assert.Equal("2026-01-01-中文标题.md", Post.BuildFileName(date, slug));
    }

    [Fact]
    public void Constructor_NullFilePath_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Post("", new FrontMatter(), "body"));
    }

    [Fact]
    public void Constructor_NullFrontMatter_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => new Post("/path/file.md", null!, "body"));
    }

    [Fact]
    public void FileName_ReturnsFileNameOnly()
    {
        var post = new Post("/posts/2026-01-01-test.md", new FrontMatter(), "");
        Assert.Equal("2026-01-01-test.md", post.FileName);
    }
}

/// <summary>
/// BlogProject 聚合根测试。
/// </summary>
public class BlogProjectTests
{
    [Fact]
    public void PostsDirectory_CombinesPathWithPosts()
    {
        var project = new BlogProject(@"C:\blog");
        Assert.Equal(Path.Combine(@"C:\blog", "_posts"), project.PostsDirectory);
    }

    [Fact]
    public void AuthorsFilePath_CombinesPathWithDataAuthorsYml()
    {
        var project = new BlogProject(@"C:\blog");
        Assert.Equal(Path.Combine(@"C:\blog", "_data", "authors.yml"), project.AuthorsFilePath);
    }
}
