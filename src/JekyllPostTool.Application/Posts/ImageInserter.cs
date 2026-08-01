using System.IO;
using System.Text;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 将本地图片复制到博文对应的资源目录，并生成 markdown 引用追加到正文末尾。
/// 依据：FR-6.2~6.6、设计方案 §4.4.1。
/// </summary>
public sealed class ImageInserter
{
    /// <summary>
    /// 将指定图片复制到 <c>{BlogProject.Path}/assets/img/{slug}/</c>，并返回应追加到正文末尾的 markdown 引用片段。
    /// </summary>
    /// <param name="project">目标博客项目。</param>
    /// <param name="postSlug">当前博文的 slug（去日期前缀）。调用方负责从文件名提取。</param>
    /// <param name="sourceImagePaths">源图片绝对路径列表。</param>
    /// <param name="alt">统一的 alt 文本，默认空字符串。</param>
    /// <returns>追加到正文末尾的 markdown 引用文本（含换行符分隔）；无成功插入时返回空字符串。</returns>
    public async Task<string> InsertAsync(
        BlogProject project,
        string postSlug,
        IReadOnlyList<string> sourceImagePaths,
        string alt = "")
    {
        ArgumentNullException.ThrowIfNull(project);
        ArgumentException.ThrowIfNullOrWhiteSpace(postSlug);

        var targetDir = Path.Combine(project.Path, "assets", "img", postSlug);
        Directory.CreateDirectory(targetDir);

        var markdownBuilder = new StringBuilder();
        foreach (var sourcePath in sourceImagePaths)
        {
            if (!File.Exists(sourcePath))
            {
                continue;
            }

            var fileName = Path.GetFileName(sourcePath);
            var destPath = Path.Combine(targetDir, fileName);

            // FR-6.4：文件名冲突自动追加 -1, -2, ... 直到可用
            destPath = ResolveConflict(destPath);

            File.Copy(sourcePath, destPath, overwrite: false);

            var destFileName = Path.GetFileName(destPath);
            markdownBuilder.AppendLine($"![{alt}](/assets/img/{postSlug}/{destFileName})");
        }

        return markdownBuilder.ToString();
    }

    /// <summary>
    /// 从博文文件名提取 slug（去日期前缀）。
    /// <example>
    /// <c>2026-07-28-writing-a-new-post.md</c> → <c>writing-a-new-post</c>
    /// </example>
    /// </summary>
    public static string ExtractSlugFromFileName(string fileName)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(fileName);

        var name = Path.GetFileNameWithoutExtension(fileName);

        // YYYY-MM-DD- 前缀
        if (name.Length >= 11 && name[4] == '-' && name[7] == '-' && name[10] == '-')
        {
            return name[11..];
        }

        return name;
    }

    private static string ResolveConflict(string destPath)
    {
        if (!File.Exists(destPath))
        {
            return destPath;
        }

        var dir = Path.GetDirectoryName(destPath) ?? string.Empty;
        var fileNameWithoutExt = Path.GetFileNameWithoutExtension(destPath);
        var ext = Path.GetExtension(destPath);

        for (var i = 1; i < int.MaxValue; i++)
        {
            var candidate = Path.Combine(dir, $"{fileNameWithoutExt}-{i}{ext}");
            if (!File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new IOException($"无法为 {destPath} 找到可用的文件名");
    }
}
