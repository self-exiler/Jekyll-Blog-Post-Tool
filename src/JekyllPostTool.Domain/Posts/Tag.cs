namespace JekyllPostTool.Domain.Posts;

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
