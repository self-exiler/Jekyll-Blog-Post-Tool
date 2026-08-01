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
}
