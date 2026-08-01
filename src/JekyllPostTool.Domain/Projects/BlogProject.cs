namespace JekyllPostTool.Domain.Projects;

/// <summary>
/// 博客项目聚合根。仅作为无状态的文件夹引用，不携带项目级配置。
/// </summary>
public sealed record BlogProject(string Path)
{
    public string PostsDirectory => System.IO.Path.Combine(Path, "_posts");

    public string AuthorsFilePath => System.IO.Path.Combine(Path, "_data", "authors.yml");
}
