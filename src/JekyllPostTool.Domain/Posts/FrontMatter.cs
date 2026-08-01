namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 博文 front matter 值对象，包含 v1 最小集强类型字段与未知字段字典。
/// </summary>
public sealed class FrontMatter
{
    public string Title { get; set; } = string.Empty;

    public DateTimeOffset? Date { get; set; }

    public IReadOnlyList<Category> Categories { get; set; } = Array.Empty<Category>();

    public IReadOnlyList<Tag> Tags { get; set; } = Array.Empty<Tag>();

    public IReadOnlyList<string> Authors { get; set; } = Array.Empty<string>();

    public string? Description { get; set; }

    /// <summary>
    /// 工具不认识的 front matter 字段，按原顺序保存以便 round-trip。
    /// </summary>
    public IDictionary<string, object?> UnknownFields { get; set; } = new OrderedDictionary<string, object?>();
}
