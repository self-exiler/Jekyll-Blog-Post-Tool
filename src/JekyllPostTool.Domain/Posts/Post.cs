namespace JekyllPostTool.Domain.Posts;

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

    public string FileName => System.IO.Path.GetFileName(FilePath);

    public static string BuildFileName(DateTimeOffset date, Slug slug)
    {
        return $"{date:yyyy-MM-dd}-{slug.Value}.md";
    }
}
