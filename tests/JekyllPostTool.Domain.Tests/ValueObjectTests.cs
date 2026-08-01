using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Domain.Tests;

/// <summary>
/// Category / Tag / Slug 值对象测试。
/// </summary>
public class ValueObjectTests
{
    [Fact]
    public void Category_EmptyValue_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Category(""));
        Assert.Throws<ArgumentException>(() => new Category("   "));
    }

    [Fact]
    public void Category_TrimsValue()
    {
        Assert.Equal("Blogging", new Category("  Blogging  ").Value);
    }

    [Fact]
    public void Tag_EmptyValue_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Tag(""));
        Assert.Throws<ArgumentException>(() => new Tag("   "));
    }

    [Fact]
    public void Tag_PreservesCase()
    {
        Assert.Equal("JavaScript", new Tag("JavaScript").Value);
        Assert.Equal("CSS", new Tag("CSS").Value);
    }

    [Fact]
    public void Tag_TrimsValue()
    {
        Assert.Equal("Writing", new Tag("  Writing  ").Value);
    }

    [Fact]
    public void Slug_EmptyValue_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Slug(""));
        Assert.Throws<ArgumentException>(() => new Slug("   "));
    }
}

/// <summary>
/// Author 实体测试。
/// </summary>
public class AuthorTests
{
    [Fact]
    public void Constructor_ValidArgs_SetsProperties()
    {
        var author = new Author("cotes", "Cotes", "@cotes", "https://example.com");
        Assert.Equal("cotes", author.Id);
        Assert.Equal("Cotes", author.Name);
        Assert.Equal("@cotes", author.Twitter);
        Assert.Equal("https://example.com", author.Url);
    }

    [Fact]
    public void Constructor_NullId_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Author("", "Name"));
        Assert.Throws<ArgumentException>(() => new Author("   ", "Name"));
    }

    [Fact]
    public void Constructor_NullName_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Author("id", ""));
        Assert.Throws<ArgumentException>(() => new Author("id", "   "));
    }

    [Fact]
    public void Constructor_WhitespaceTwitter_TreatedAsNull()
    {
        var author = new Author("id", "Name", "   ", "   ");
        Assert.Null(author.Twitter);
        Assert.Null(author.Url);
    }

    [Fact]
    public void Constructor_TrimsFields()
    {
        var author = new Author("  id  ", "  Name  ", "  @tw  ", "  https://url  ");
        Assert.Equal("id", author.Id);
        Assert.Equal("Name", author.Name);
        Assert.Equal("@tw", author.Twitter);
        Assert.Equal("https://url", author.Url);
    }
}
