namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 从 title 生成的文件名片段值对象。
/// </summary>
public sealed record Slug(string Value)
{
    public string Value { get; } = string.IsNullOrWhiteSpace(Value)
        ? throw new ArgumentException("Slug 不能为空", nameof(Value))
        : Value.Trim();

    public override string ToString() => Value;
}

/// <summary>
/// 分类值对象，原样保留用户输入。
/// </summary>
public sealed record Category(string Value)
{
    public string Value { get; } = string.IsNullOrWhiteSpace(Value)
        ? throw new ArgumentException("Category 不能为空", nameof(Value))
        : Value.Trim();

    public override string ToString() => Value;
}

/// <summary>
/// 标签值对象，原样保留大小写与中英文。
/// </summary>
public sealed record Tag(string Value)
{
    public string Value { get; } = string.IsNullOrWhiteSpace(Value)
        ? throw new ArgumentException("Tag 不能为空", nameof(Value))
        : Value.Trim();

    public override string ToString() => Value;
}
