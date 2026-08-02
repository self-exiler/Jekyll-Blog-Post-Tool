using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Yaml;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 构造博文 front matter 与预览文本的纯函数集合。
/// 从 PostPageViewModel 抽出，便于单元测试。
/// </summary>
public static class FrontMatterBuilder
{
    /// <summary>
    /// 由编辑状态构造 FrontMatter（含日期、分类、标签、作者、描述）。
    /// </summary>
    public static FrontMatter Build(PostEditState state)
    {
        var categories = new List<Category>();
        if (!string.IsNullOrWhiteSpace(state.Category1))
        {
            categories.Add(new Category(state.Category1));
        }

        if (!string.IsNullOrWhiteSpace(state.Category2))
        {
            categories.Add(new Category(state.Category2));
        }

        // 标签输入用空格分隔；生成博文时序列化为 YAML 数组（标准格式）
        var tags = state.Tags
            .Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Where(t => !string.IsNullOrWhiteSpace(t))
            .Select(t => new Tag(t))
            .ToList();

        var date = ComputeDate(state.SelectedDate, state.SelectedTime, state.SelectedTimeZone);

        return new FrontMatter
        {
            Title = state.Title,
            Date = date,
            Categories = categories,
            Tags = tags,
            Authors = state.SelectedAuthorIds,
            Description = string.IsNullOrWhiteSpace(state.Description) ? null : state.Description
        };
    }

    /// <summary>
    /// 由选择日期、时间、时区计算博文 date 字段；SelectedDate 为 null 返回 null。
    /// </summary>
    public static DateTimeOffset? ComputeDate(DateTimeOffset? selectedDate, TimeSpan selectedTime, string? selectedTimeZone)
    {
        if (!selectedDate.HasValue)
        {
            return null;
        }

        var offset = TimeZoneFormatter.ParseOrLocal(selectedTimeZone);
        var localDate = selectedDate.Value.Date.Add(selectedTime);
        return new DateTimeOffset(localDate, offset);
    }

    /// <summary>
    /// 生成文件名预览与 front matter 预览文本；构造异常时回退到占位文案。
    /// </summary>
    public static (string FileNamePreview, string FrontMatterPreview) BuildPreview(FrontMatter frontMatter)
    {
        try
        {
            var slug = SlugGenerator.Generate(frontMatter.Title);
            var fileName = frontMatter.Date.HasValue
                ? Post.BuildFileName(frontMatter.Date.Value, slug)
                : $"{slug.Value}.md";

            var yaml = YamlFrontMatterSerializer.Serialize(frontMatter);
            var preview = $"---{Environment.NewLine}{yaml}{Environment.NewLine}---";
            return (fileName, preview);
        }
        catch
        {
            return ("填写标题后生成文件名", string.Empty);
        }
    }
}

/// <summary>
/// 用于构造 FrontMatter 的编辑状态快照（值对象）。
/// </summary>
public sealed record PostEditState(
    string Title,
    string Category1,
    string Category2,
    string Tags,
    string Description,
    DateTimeOffset? SelectedDate,
    TimeSpan SelectedTime,
    string SelectedTimeZone,
    IReadOnlyList<string> SelectedAuthorIds);
