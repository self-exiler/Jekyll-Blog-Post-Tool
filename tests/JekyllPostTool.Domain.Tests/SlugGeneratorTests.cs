using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Domain.Tests;

/// <summary>
/// SlugGenerator 单元测试（ADR-006: 中文保留、英文小写、符号压缩）。
/// </summary>
public class SlugGeneratorTests
{
    [Theory]
    [InlineData("Hello World", "hello-world")]
    [InlineData("Writing a New Post", "writing-a-new-post")]
    [InlineData("  Multiple   Spaces  ", "multiple-spaces")]
    [InlineData("Title-With-Hyphens", "title-with-hyphens")]
    [InlineData("Title_With_Underscores", "title-with-underscores")]
    [InlineData("UPPERCASE", "uppercase")]
    [InlineData("Mixed CASE Title", "mixed-case-title")]
    public void Generate_EnglishTitle_LowercasesAndHyphenates(string title, string expected)
    {
        var slug = SlugGenerator.Generate(title);
        Assert.Equal(expected, slug.Value);
    }

    [Theory]
    [InlineData("中文标题", "中文标题")]
    [InlineData("Hello 世界", "hello-世界")]
    [InlineData("写作新文章", "写作新文章")]
    [InlineData("Jekyll 博客工具", "jekyll-博客工具")]
    public void Generate_ChineseTitle_PreservesChinese(string title, string expected)
    {
        var slug = SlugGenerator.Generate(title);
        Assert.Equal(expected, slug.Value);
    }

    [Theory]
    [InlineData("Title: With Special! @Chars#", "title-with-special-chars")]
    [InlineData("A.B.C", "a-b-c")]
    [InlineData("Title (With Parens)", "title-with-parens")]
    [InlineData("价格/折扣", "价格-折扣")]
    public void Generate_SpecialCharacters_CompressesToHyphens(string title, string expected)
    {
        var slug = SlugGenerator.Generate(title);
        Assert.Equal(expected, slug.Value);
    }

    [Theory]
    [InlineData("---Leading Hyphens", "leading-hyphens")]
    [InlineData("Trailing Hyphens---", "trailing-hyphens")]
    [InlineData("###Edge###", "edge")]
    public void Generate_LeadingTrailingHyphens_Trimmed(string title, string expected)
    {
        var slug = SlugGenerator.Generate(title);
        Assert.Equal(expected, slug.Value);
    }

    [Fact]
    public void Generate_TitleWithNumbers_PreservesNumbers()
    {
        Assert.Equal("post-2026", SlugGenerator.Generate("Post 2026").Value);
        Assert.Equal("version-2-0", SlugGenerator.Generate("Version 2.0").Value);
    }

    [Fact]
    public void Generate_AllSymbolsTitle_ReturnsUntitled()
    {
        Assert.Equal("untitled", SlugGenerator.Generate("---").Value);
        Assert.Equal("untitled", SlugGenerator.Generate("!!!").Value);
    }

    [Fact]
    public void Generate_NullOrWhiteSpace_Throws()
    {
        Assert.ThrowsAny<ArgumentException>(() => SlugGenerator.Generate(null!));
        Assert.ThrowsAny<ArgumentException>(() => SlugGenerator.Generate(""));
        Assert.ThrowsAny<ArgumentException>(() => SlugGenerator.Generate("   "));
    }

    [Fact]
    public void Generate_ConsecutiveHyphens_CompressedToOne()
    {
        var slug = SlugGenerator.Generate("A---B---C");
        Assert.Equal("a-b-c", slug.Value);
    }

    [Fact]
    public void Generate_MixedChineseEnglishNumbers()
    {
        var slug = SlugGenerator.Generate("Jekyll 3.0 博客工具");
        Assert.Equal("jekyll-3-0-博客工具", slug.Value);
    }
}
