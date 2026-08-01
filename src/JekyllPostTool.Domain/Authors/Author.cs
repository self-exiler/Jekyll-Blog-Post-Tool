namespace JekyllPostTool.Domain.Authors;

/// <summary>
/// 作者实体，按项目隔离，存于 _data/authors.yml。
/// </summary>
public sealed class Author
{
    public string Id { get; }

    public string Name { get; }

    public string? Twitter { get; }

    public string? Url { get; }

    public Author(string id, string name, string? twitter = null, string? url = null)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(id);
        ArgumentException.ThrowIfNullOrWhiteSpace(name);

        Id = id.Trim();
        Name = name.Trim();
        Twitter = string.IsNullOrWhiteSpace(twitter) ? null : twitter.Trim();
        Url = string.IsNullOrWhiteSpace(url) ? null : url.Trim();
    }
}
