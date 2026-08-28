namespace JekyllPostTool.Domain.Posts;

using System.Globalization;

/// <summary>
/// 博文实体，包含文件路径、front matter 与正文引用。
/// </summary>
public sealed class Post
{
    public string FilePath { get; }

    public FrontMatter FrontMatter { get; }

    public string Body { get; }

    public Post(string filePath, FrontMatter frontMatter, string body)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(filePath);
        ArgumentNullException.ThrowIfNull(frontMatter);

        FilePath = filePath;
        FrontMatter = frontMatter;
        Body = body;
    }

    public string FileName => Path.GetFileName(FilePath);

    /// <summary>
    /// 由日期与 slug 构造博文文件名（YYYY-MM-DD-slug.md）。Invariant：文件名不得含本地化数字。
    /// </summary>
    public static string BuildFileName(DateTimeOffset date, Slug slug)
    {
        return string.Create(CultureInfo.InvariantCulture, $"{date:yyyy-MM-dd}-{slug.Value}.md");
    }

    /// <summary>
    /// 从博文文件名提取 slug 的逆变换（<see cref="BuildFileName"/> 的对偶）。
    /// 无合法 YYYY-MM-DD- 前缀时返回去掉扩展名的原文件名。
    /// </summary>
    /// <example>
    /// <c>2026-07-28-writing-a-new-post.md</c> → <c>writing-a-new-post</c>
    /// </example>
    public static string TryExtractSlug(string fileName)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(fileName);

        var name = Path.GetFileNameWithoutExtension(fileName);

        // 前缀须为数字年(4)-月(2)-日(2)，且后缀非空；避免误剥标题恰似该形态的文件名
        if (name.Length >= 12
            && char.IsDigit(name[0]) && char.IsDigit(name[1]) && char.IsDigit(name[2]) && char.IsDigit(name[3])
            && name[4] == '-'
            && char.IsDigit(name[5]) && char.IsDigit(name[6])
            && name[7] == '-'
            && char.IsDigit(name[8]) && char.IsDigit(name[9])
            && name[10] == '-')
        {
            return name[11..];
        }

        return name;
    }
}
