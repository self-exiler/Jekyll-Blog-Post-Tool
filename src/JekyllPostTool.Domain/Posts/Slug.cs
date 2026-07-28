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
