namespace JekyllPostTool.Domain.Authors;

/// <summary>
/// 作者实体，按项目隔离，存于 _data/authors.yml。
/// </summary>
public sealed class Author : IEquatable<Author>
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

    public Author With(string? name = null, string? twitter = null, string? url = null)
    {
        return new Author(
            Id,
            name ?? Name,
            twitter ?? Twitter,
            url ?? Url);
    }

    public bool Equals(Author? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id;
    }

    public override bool Equals(object? obj) => obj is Author other && Equals(other);

    public override int GetHashCode() => Id.GetHashCode(StringComparison.Ordinal);
}
