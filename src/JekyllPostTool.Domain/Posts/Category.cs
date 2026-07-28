namespace JekyllPostTool.Domain.Posts;

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
