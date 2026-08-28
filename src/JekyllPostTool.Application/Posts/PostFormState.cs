using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 博文表单编辑态快照，「表单 ↔ Front Matter」双向映射的唯一权威：
/// 标签空格分隔、categories 两格拆分、date 拆为日期/时间/时区三格的规则只在此处定义。
/// 纯值对象无 UI 依赖——<see cref="FromFrontMatter"/> 与 <see cref="ToFrontMatter"/> 的 round-trip 即测试面。
/// </summary>
public sealed record PostFormState(
    string Title,
    DateTimeOffset? SelectedDate,
    TimeSpan SelectedTime,
    string SelectedTimeZone,
    string Category1,
    string Category2,
    string Tags,
    string Description,
    IReadOnlyList<string> SelectedAuthorIds)
{
    /// <summary>新建博文时的空白状态（FR-3.1：date 不预填默认值）。</summary>
    public static PostFormState Empty(DateTimeOffset now) => new(
        Title: string.Empty,
        SelectedDate: null,
        SelectedTime: now.TimeOfDay,
        SelectedTimeZone: TimeZoneFormatter.Format(now.Offset),
        Category1: string.Empty,
        Category2: string.Empty,
        Tags: string.Empty,
        Description: string.Empty,
        SelectedAuthorIds: []);

    /// <summary>Front Matter → 表单。</summary>
    public static PostFormState FromFrontMatter(FrontMatter frontMatter)
    {
        DateTimeOffset? selectedDate = null;
        var selectedTime = TimeSpan.Zero;
        var selectedTimeZone = string.Empty;
        if (frontMatter.Date.HasValue)
        {
            var date = frontMatter.Date.Value;
            selectedDate = date;
            selectedTime = date.DateTime.TimeOfDay;
            selectedTimeZone = TimeZoneFormatter.Format(date.Offset);
        }

        return new PostFormState(
            Title: frontMatter.Title,
            SelectedDate: selectedDate,
            SelectedTime: selectedTime,
            SelectedTimeZone: selectedTimeZone,
            Category1: frontMatter.Categories.ElementAtOrDefault(0)?.Value ?? string.Empty,
            Category2: frontMatter.Categories.ElementAtOrDefault(1)?.Value ?? string.Empty,
            // 标签输入用空格分隔（与 ToFrontMatter 的切分规则对偶）
            Tags: string.Join(" ", frontMatter.Tags.Select(t => t.Value)),
            Description: frontMatter.Description ?? string.Empty,
            SelectedAuthorIds: [.. frontMatter.Authors]);
    }

    /// <summary>表单 → Front Matter（title/date 缺失等校验由 FrontMatterValidator 负责）。</summary>
    public FrontMatter ToFrontMatter()
    {
        var categories = new List<Category>();
        if (!string.IsNullOrWhiteSpace(Category1))
        {
            categories.Add(new Category(Category1));
        }

        if (!string.IsNullOrWhiteSpace(Category2))
        {
            categories.Add(new Category(Category2));
        }

        var tags = Tags
            .Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Select(t => new Tag(t))
            .ToList();

        DateTimeOffset? date = null;
        if (SelectedDate.HasValue)
        {
            date = new DateTimeOffset(SelectedDate.Value.Date.Add(SelectedTime), TimeZoneFormatter.ParseOrLocal(SelectedTimeZone));
        }

        return new FrontMatter
        {
            Title = Title,
            Date = date,
            Categories = categories,
            Tags = tags,
            Authors = SelectedAuthorIds,
            Description = string.IsNullOrWhiteSpace(Description) ? null : Description
        };
    }
}
