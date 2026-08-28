using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// PostFormState「表单 ↔ Front Matter」映射的 round-trip 测试。
/// 标签分隔、categories 两格、date 三合一重组的唯一权威在这里验证。
/// </summary>
public class PostFormStateTests
{
    [Fact]
    public void Empty_HasNoDateAndNoTitle()
    {
        var now = new DateTimeOffset(2026, 8, 25, 12, 0, 0, TimeSpan.FromHours(8));

        var state = PostFormState.Empty(now);

        Assert.Equal(string.Empty, state.Title);
        Assert.Null(state.SelectedDate);
        Assert.Equal(now.TimeOfDay, state.SelectedTime);
        Assert.Equal("+08:00", state.SelectedTimeZone);
    }

    [Fact]
    public void ToFrontMatter_SplitsTagsBySpace()
    {
        var state = PostFormState.Empty(DateTimeOffset.Now) with { Title = "t", Tags = "a b  c" };

        var fm = state.ToFrontMatter();
        Assert.Equal(["a", "b", "c"], fm.Tags.Select(t => t.Value).ToArray());
    }

    [Fact]
    public void FromFrontMatter_JoinsTagsWithSpace()
    {
        var fm = new FrontMatter
        {
            Title = "t",
            Tags = new[] { new Tag("a"), new Tag("b") }
        };

        var state = PostFormState.FromFrontMatter(fm);

        Assert.Equal("a b", state.Tags);
    }

    [Fact]
    public void RoundTrip_PreservesCategoriesDateAndAuthors()
    {
        var fm = new FrontMatter
        {
            Title = "标题",
            Date = new DateTimeOffset(2026, 7, 28, 9, 30, 0, TimeSpan.FromHours(8)),
            Categories = new[] { new Category("博客"), new Category("技术") },
            Tags = new[] { new Tag("jekyll") },
            Authors = new[] { "cotes" },
            Description = "描述"
        };

        // 表单 → Front Matter → 表单 的 round-trip 保持字段一致（ADR-007）
        var state = PostFormState.FromFrontMatter(fm);
        var rebuilt = state.ToFrontMatter();

        Assert.Equal(fm.Title, rebuilt.Title);
        Assert.Equal(fm.Date, rebuilt.Date);
        Assert.Equal(fm.Categories.Select(c => c.Value), rebuilt.Categories.Select(c => c.Value));
        Assert.Equal(fm.Tags.Select(t => t.Value), rebuilt.Tags.Select(t => t.Value));
        Assert.Equal(fm.Authors, rebuilt.Authors);
        Assert.Equal(fm.Description, rebuilt.Description);

        Assert.Equal("2026-07-28", state.SelectedDate!.Value.ToString("yyyy-MM-dd"));
        Assert.Equal(new TimeSpan(9, 30, 0), state.SelectedTime);
        Assert.Equal("+08:00", state.SelectedTimeZone);
        Assert.Equal("博客", state.Category1);
        Assert.Equal("技术", state.Category2);
    }

    [Fact]
    public void ToFrontMatter_NoSelectedDate_DateStaysNull()
    {
        // FR-3.1：新建时 date 为空，不预填默认值
        var state = PostFormState.Empty(DateTimeOffset.Now) with { Title = "t" };
        var fm = state.ToFrontMatter();

        Assert.False(fm.Date.HasValue);
    }

    [Fact]
    public void ToFrontMatter_ComposesDateFromThreeFieldsWithParsedOffset()
    {
        var state = PostFormState.Empty(DateTimeOffset.Now) with
        {
            Title = "t",
            SelectedDate = new DateTimeOffset(2026, 7, 28, 0, 0, 0, TimeSpan.Zero),
            SelectedTime = new TimeSpan(15, 5, 0),
            SelectedTimeZone = "-05:00"
        };

        var fm = state.ToFrontMatter();

        Assert.Equal(new DateTimeOffset(2026, 7, 28, 15, 5, 0, TimeSpan.FromHours(-5)), fm.Date);
    }
}
