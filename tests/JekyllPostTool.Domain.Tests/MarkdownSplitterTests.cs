using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Domain.Tests;

/// <summary>
/// MarkdownSplitter 切分测试。
/// </summary>
public class MarkdownSplitterTests
{
    [Fact]
    public void TrySplit_ValidFrontMatter_SplitsCorrectly()
    {
        var content = "---\ntitle: Hello\ndate: 2026-07-28\n---\nThis is the body.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Hello", yaml);
        Assert.Contains("date: 2026-07-28", yaml);
        Assert.Equal("This is the body.", body);
    }

    [Fact]
    public void TrySplit_NoFrontMatter_ReturnsFalse()
    {
        var content = "Just some text without front matter.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.False(result);
        Assert.Equal(string.Empty, yaml);
        Assert.Equal(content, body);
    }

    [Fact]
    public void TrySplit_OnlyOpeningDelimiter_ReturnsFalse()
    {
        var content = "---\ntitle: Hello\nNo closing delimiter";

        var result = MarkdownSplitter.TrySplit(content, out _, out _);

        Assert.False(result);
    }

    [Fact]
    public void TrySplit_EmptyBody_ReturnsEmptyString()
    {
        var content = "---\ntitle: Hello\n---\n";

        var result = MarkdownSplitter.TrySplit(content, out _, out var body);

        Assert.True(result);
        Assert.Equal(string.Empty, body);
    }

    [Fact]
    public void TrySplit_LeadingWhitespace_HandledCorrectly()
    {
        var content = "\n\n---\ntitle: Hello\n---\nBody here.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Hello", yaml);
        Assert.Equal("Body here.", body);
    }

    [Fact]
    public void TrySplit_BodyContainsHorizontalRule_DoesNotMistakeAsClosingDelimiter()
    {
        // 正文中包含独立的 ---（markdown 水平分割线），不应被误判为闭定界符
        var content = "---\ntitle: Hello\n---\nIntro paragraph.\n\n---\n\nMore body text.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Hello", yaml);
        Assert.StartsWith("Intro paragraph.", body);
        Assert.Contains("---", body);
        Assert.Contains("More body text.", body);
    }

    [Fact]
    public void TrySplit_BodyLineStartingWithHyphens_DoesNotMatchAsDelimiter()
    {
        // "---" 必须独占一行才匹配；行内含 "---" 但前后有其他字符的不匹配
        var content = "---\ntitle: Test\n---\n---Some text---";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Test", yaml);
        Assert.Equal("---Some text---", body);
    }

    [Fact]
    public void TrySplit_ClosingDelimiterWithTrailingSpaces_StillMatches()
    {
        // 闭定界符行尾带空格也应匹配
        var content = "---\ntitle: Hello\n---   \nBody.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Hello", yaml);
        Assert.Equal("Body.", body);
    }

    [Fact]
    public void TrySplit_YamlValueContainsDashesInValue_DoesNotMistakeAsDelimiter()
    {
        // YAML 值中含 "---" 但非独占一行，不应被误判
        var content = "---\ntitle: A---B\n---\nBody.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: A---B", yaml);
        Assert.Equal("Body.", body);
    }

    [Fact]
    public void TrySplit_EmptyContent_ReturnsFalse()
    {
        var result = MarkdownSplitter.TrySplit("", out var yaml, out var body);

        Assert.False(result);
        Assert.Equal(string.Empty, yaml);
        Assert.Equal("", body);
    }

    [Fact]
    public void TrySplit_CrllLineEndings_HandledCorrectly()
    {
        var content = "---\r\ntitle: Hello\r\n---\r\nBody here.";

        var result = MarkdownSplitter.TrySplit(content, out var yaml, out var body);

        Assert.True(result);
        Assert.Contains("title: Hello", yaml);
        Assert.Equal("Body here.", body);
    }
}
