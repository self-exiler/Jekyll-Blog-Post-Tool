namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 博文仓储接口。
/// </summary>
public interface IPostRepository
{
    bool Exists(string filePath);

    void Delete(string filePath);

    /// <summary>单次读盘：原文与解析结果同源，保证外部修改检测的基线哈希与展示内容一致；文件不存在返回 null。</summary>
    Task<PostRead?> ReadAsync(string filePath, CancellationToken cancellationToken = default);

    Task SaveAsync(Post post, CancellationToken cancellationToken = default);

    /// <summary>读取原始全文（不解析）；文件不存在返回 null。</summary>
    Task<string?> ReadAllTextAsync(string filePath, CancellationToken cancellationToken = default);
}

/// <summary>博文文件的读盘快照：全文与解析结果来自同一次 IO。</summary>
public sealed record PostRead(string FilePath, string Content, FrontMatter FrontMatter, string Body)
{
    public Post ToPost() => new(FilePath, FrontMatter, Body);
}
