using System.IO;
using System.Text;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Infrastructure.FileSystem;

/// <summary>
/// 将本地图片复制到博文对应的资源目录，并生成 markdown 引用插入正文（内存，保存时统一写入）。
/// 依据：FR-6.2~6.6、设计方案 §4.4.1。
/// </summary>
public sealed class ImageInserter
{
    /// <summary>
    /// 将指定图片异步复制到 <c>{BlogProject.Path}/assets/img/{slug}/</c>，返回 markdown 引用片段。
    /// slug 从文件名提取见 <see cref="Post.TryExtractSlug"/>（Domain）。
    /// </summary>
    /// <param name="project">目标博客项目。</param>
    /// <param name="postSlug">当前博文的 slug（去日期前缀）。</param>
    /// <param name="sourceImagePaths">源图片绝对路径列表。</param>
    /// <param name="alt">统一的 alt 文本，默认空字符串。</param>
    /// <returns>markdown 引用文本（换行分隔）；无成功复制时为空字符串。</returns>
    public async Task<string> InsertAsync(
        BlogProject project,
        string postSlug,
        IReadOnlyList<string> sourceImagePaths,
        string alt = "",
        CancellationToken cancellationToken = default)
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

            // 流式异步复制，大图不再阻塞调用线程
            await using (var source = File.OpenRead(sourcePath))
            await using (var destination = File.Create(destPath))
            {
                await source.CopyToAsync(destination, cancellationToken);
            }

            var destFileName = Path.GetFileName(destPath);
            markdownBuilder.AppendLine($"![{EscapeAlt(alt)}](/assets/img/{postSlug}/{destFileName})");
        }

        return markdownBuilder.ToString();
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

    /// <summary>转义 alt 文本中的 markdown 链接语法字符，避免生成断裂的引用。</summary>
    private static string EscapeAlt(string alt) => alt
        .Replace("[", "\\[")
        .Replace("]", "\\]")
        .Replace("(", "\\(")
        .Replace(")", "\\)");
}
